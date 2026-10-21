#![no_std]
extern crate alloc;
use data_serialization::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serialization(deny_unknown_fields)]
pub struct Record {
    pub name: alloc::string::String,
    pub value: alloc::rc::Rc<u32>,
}
pub fn encode(value: &Record) -> alloc::string::String {
    json_codec::to_string(value).unwrap()
}
