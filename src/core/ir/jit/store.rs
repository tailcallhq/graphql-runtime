use std::collections::HashMap;

use serde_json_borrow::Value;

use crate::core::ir::jit::model::FieldId;

#[allow(unused)]
pub struct Store {
    map: HashMap<FieldId, Data<'static>>,
}
#[allow(unused)]
#[derive(Clone)]
pub enum Data<'a> {
    Value(Value<'a>),
    List(Vec<Value<'a>>),
}
#[allow(unused)]
impl Data<'_> {
    pub fn as_value(&self) -> Option<&Value> {
        match self {
            Data::Value(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<Value>> {
        match self {
            Data::List(list) => Some(list),
            _ => None,
        }
    }
}

#[allow(unused)]
impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    pub fn new() -> Self {
        Store { map: HashMap::new() }
    }

    pub fn insert(&mut self, field_id: FieldId, data: Data<'static>) {
        self.map.insert(field_id, data);
    }

    pub fn get(&self, field_id: &FieldId) -> Option<&Data> {
        self.map.get(field_id)
    }
}
