use serde_json_borrow::Value;

use crate::core::ir::jit::model::FieldId;

#[allow(unused)]
pub struct Store {
    map: Vec<Data<'static>>,
}
#[allow(unused)]
#[derive(Clone)]
pub enum Data<'a> {
    /// Represents that the value was computed only once for the associated
    /// field
    Single(Value<'a>),
    /// Represents that the value was computed multiple times for the associated
    /// field. The order is guaranteed by the executor to be the same as the
    /// other of invocation and not the other of completion.
    Multiple(Vec<Value<'a>>),
}
#[allow(unused)]
impl Data<'_> {
    pub fn as_value(&self) -> Option<&Value> {
        match self {
            Data::Single(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<Value>> {
        match self {
            Data::Multiple(list) => Some(list),
            _ => None,
        }
    }
}

impl Store {
    pub fn new(size: usize) -> Self {
        let map = (0..size).map(|_| Data::Single(Value::Null)).collect();
        Store { map }
    }

    pub fn set(&mut self, field_id: FieldId, data: Data<'static>) {
        self.map.insert(field_id.as_usize(), data);
    }

    pub fn get(&self, field_id: &FieldId) -> Option<&Data> {
        self.map.get(field_id.as_usize())
    }
}
