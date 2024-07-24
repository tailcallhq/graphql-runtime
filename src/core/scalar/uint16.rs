use schemars::JsonSchema;
use tailcall_macros::ScalarDefinition;

use crate::core::json::JsonLikeOwned;

/// Represents unsigned integer type 16bit size
#[derive(JsonSchema, Default, ScalarDefinition)]
pub struct UInt16(pub u16);

impl super::Scalar for UInt16 {
    fn validate<Value: JsonLikeOwned>(&self) -> fn(&Value) -> bool {
        |value| value.as_u64().map_or(false, |n| u16::try_from(n).is_ok())
    }
}

#[cfg(test)]
mod test {
    use async_graphql_value::ConstValue;
    use serde_json::Number;

    use super::UInt16;
    use crate::core::scalar::Scalar;
    use crate::{test_scalar_invalid, test_scalar_valid};

    test_scalar_valid! {
        UInt16,
        ConstValue::Number(Number::from(100u32)),
        ConstValue::Number(Number::from(2 * u8::MAX as u64))
    }

    test_scalar_invalid! {
        UInt16,
        ConstValue::Null,
        ConstValue::Number(Number::from(u16::MAX as u64 + 1)),
        ConstValue::Number(Number::from(-1)),
        ConstValue::Number(
            Number::from_f64(1.25).unwrap()
        ),
        ConstValue::String("4564846".to_string())
    }
}
