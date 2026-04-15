use crate::{Error};
use js_sys::{Error as JsError};
use uuid::Uuid;
use wasm_bindgen::prelude::*;

impl From<JsValue> for Error{
    fn from(err: JsValue) -> Self {
        Error::JavaScript(JsError::from(err).message().into())
    }
}

pub fn uuid_from_string(uuid: String) -> Uuid {
    Uuid::parse_str(&uuid).unwrap()
}
