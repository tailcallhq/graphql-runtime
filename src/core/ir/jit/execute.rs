use serde_json_borrow::OwnedValue;

use crate::core::ir::{EvaluationError, IR};

#[allow(unused)]
pub async fn execute_ir(
    ir: &IR,
    parent: Option<&OwnedValue>,
) -> Result<OwnedValue, EvaluationError> {
    todo!()
}
