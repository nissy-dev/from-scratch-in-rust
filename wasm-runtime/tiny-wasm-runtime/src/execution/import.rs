use std::collections::HashMap;

use super::{store::Store, value::Value};
use anyhow::Result;

pub type ImportFunc = Box<dyn FnMut(&mut Store, Vec<Value>) -> Result<Option<Value>>>;
pub type Import = HashMap<String, HashMap<String, ImportFunc>>;
