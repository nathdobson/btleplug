use super::peripheral::{Peripheral, PeripheralId};
use crate::api::CentralState;
use crate::api::{Central, CentralEvent, Peripheral as _, ScanFilter};
use crate::common::adapter_manager::AdapterManager;
use crate::{Error, Result};
use async_trait::async_trait;
use futures::channel::oneshot;
use futures::stream::Stream;
use js_sys::JsString;
use std::pin::Pin;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{BluetoothDevice, BluetoothLeScanFilterInit, RequestDeviceOptions};

fn spawn_and_join_local<F: 'static + Future>(f: F) -> impl Future<Output = F::Output> {
    let (sender, receiver) = oneshot::channel();
    spawn_local(async move {
        let _ = sender.send(f.await);
    });
    async move { receiver.await.unwrap() }
}

/// Implementation of [api::Central](crate::api::Central).
#[derive(Clone, Debug)]
pub struct Adapter {
    manager: Arc<AdapterManager<Peripheral>>,
}

fn bluetooth() -> Result<web_sys::Bluetooth> {
    Ok(web_sys::window()
        .ok_or(Error::NoAdapterAvailable)?
        .navigator()
        .bluetooth()
        .ok_or(Error::NoAdapterAvailable)?)
}

impl AdapterManager<Peripheral> {
    pub async fn add_initial_peripherals(self: &Arc<Self>) -> Result<Vec<PeripheralId>> {
        if !self.peripherals().is_empty() {
            return Ok(vec![]);
        }

        let self_clone = self.clone();
        let result = spawn_and_join_local(async move {
            let bluetooth = bluetooth()?;
            // This method is not present on Chrome, so ignore it.
            let Ok(devices) = bluetooth.get_devices() else {
                return Ok(vec![]);
            };
            Result::Ok(devices.await.map_or(vec![], |devices| {
                devices
                    .iter()
                    .map(|device| self_clone.add_device(device).unwrap())
                    .collect()
            }))
        })
        .await?;
        Ok(result)
    }

    fn add_device(self: &Arc<Self>, device: BluetoothDevice) -> Option<PeripheralId> {
        let p = Peripheral::new(Arc::downgrade(self), device);
        let id = p.id();
        if self.peripheral(&id).is_none() {
            self.add_peripheral(p);
            Some(id)
        } else {
            None
        }
    }
}

impl Adapter {
    pub(crate) fn try_new() -> Result<Self> {
        bluetooth()?;
        Ok(Self {
            manager: Arc::new(AdapterManager::default()),
        })
    }
}

#[async_trait]
impl Central for Adapter {
    type Peripheral = Peripheral;

    async fn events(&self) -> Result<Pin<Box<dyn Stream<Item = CentralEvent> + Send>>> {
        Ok(self.manager.event_stream())
    }

    async fn start_scan(&self, filter: ScanFilter) -> Result<()> {
        let manager = self.manager.clone();
        spawn_and_join_local(async move {
            for id in manager.add_initial_peripherals().await? {
                manager.emit(CentralEvent::DeviceDiscovered(id));
            }

            let mut options = RequestDeviceOptions::new();
            let mut optional_services = Vec::<JsString>::new();
            let mut filters = Vec::<BluetoothLeScanFilterInit>::new();

            for uuid in filter.services.iter() {
                let mut filter = BluetoothLeScanFilterInit::new();
                let mut filter_services = Vec::<JsString>::new();
                filter_services.push(uuid.to_string().into());
                filter.services(&filter_services);
                filters.push(filter);
                optional_services.push(uuid.to_string().into());
            }

            options.filters(&filters);
            options.optional_services(&optional_services);

            let device = bluetooth().unwrap().request_device(&options).await?;

            if let Some(id) = manager.add_device(device) {
                manager.emit(CentralEvent::DeviceDiscovered(id));
            }
            Ok::<_, Error>(())
        })
        .await
    }

    async fn stop_scan(&self) -> Result<()> {
        Ok(())
    }

    async fn peripherals(&self) -> Result<Vec<Peripheral>> {
        self.manager.add_initial_peripherals().await?;
        Ok(self.manager.peripherals())
    }

    async fn peripheral(&self, id: &PeripheralId) -> Result<Peripheral> {
        self.manager.add_initial_peripherals().await?;
        self.manager.peripheral(id).ok_or(Error::DeviceNotFound)
    }

    async fn add_peripheral(&self, address: &PeripheralId) -> crate::Result<Self::Peripheral> {
        Err(Error::NotSupported(
            "Can't add a Peripheral from a BDAddr".to_string(),
        ))
    }

    async fn adapter_info(&self) -> Result<String> {
        Ok("WebBluetooth".to_string())
    }

    async fn clear_peripherals(&self) -> crate::Result<()> {
        todo!();
    }
    async fn adapter_state(&self) -> crate::Result<CentralState> {
        todo!();
    }
}
