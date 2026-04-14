use crate::{Error, Result};
use js_sys::{Error as JsError, Promise};
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

impl From<JsValue> for Error{
    fn from(err: JsValue) -> Self {
        Error::JavaScript(JsError::from(err).message().into())
    }
}

pub fn uuid_from_string(uuid: String) -> Uuid {
    Uuid::parse_str(&uuid).unwrap()
}
