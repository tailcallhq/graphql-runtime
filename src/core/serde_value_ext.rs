use std::borrow::Cow;

use serde::de::DeserializeOwned;

use crate::core::blueprint::DynamicValue;
use crate::core::path::PathString;

use crate::core::json::{JsonLike, JsonObjectLike};

pub trait ValueExt {
    type Output;
    fn render_value(&self, ctx: &impl PathString) -> Self::Output;
}

impl<A: for<'a> JsonLike<'a> + DeserializeOwned + Clone> ValueExt for DynamicValue<A> {
    type Output = A;
    fn render_value(&self, ctx: &impl PathString) -> Self::Output {
        match self {
            DynamicValue::Mustache(m) => {
                let rendered = m.render(ctx);
                serde_json::from_str::<A>(rendered.as_ref())
                    // parsing can fail when Mustache::render returns bare string and since
                    // that string is not wrapped with quotes serde_json will fail to parse it
                    // but, we can just use that string as is
                    .unwrap_or_else(|_| JsonLike::string(Cow::Owned(rendered)))
            }
            DynamicValue::Value(v) => v.clone(),
            DynamicValue::Object(obj) => {
                let mut storage = A::JsonObject::with_capacity(obj.len());
                for (key, value) in obj.iter() {
                    let key = key.as_str();
                    let value = value.render_value(ctx);
                    storage.insert_key(key, value);
                }

                A::object(storage)
            }
            DynamicValue::Array(arr) => {
                let out: Vec<_> = arr.iter().map(|v| v.render_value(ctx)).collect();
                A::array(out)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::core::blueprint::DynamicValue;
    use crate::core::serde_value_ext::ValueExt;

    #[test]
    fn test_render_value() {
        let value = json!({"a": "{{foo}}"});
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": {"bar": "baz"}});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected = async_graphql::Value::from_json(json!({"a": {"bar": "baz"}})).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_render_value_nested() {
        let value = json!({"a": "{{foo.bar.baz}}"});
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": {"bar": {"baz": 1}}});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected = async_graphql::Value::from_json(json!({"a": 1})).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_render_value_nested_str() {
        let value = json!({"a": "{{foo.bar.baz}}"});
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": {"bar": {"baz": "foo"}}});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected = async_graphql::Value::from_json(json!({"a": "foo"})).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_render_value_null() {
        let value = json!("{{foo.bar.baz}}");
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": {"bar": {"baz": null}}});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected = async_graphql::Value::from_json(json!(null)).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_render_value_nested_bool() {
        let value = json!({"a": "{{foo.bar.baz}}"});
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": {"bar": {"baz": true}}});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected = async_graphql::Value::from_json(json!({"a": true})).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_render_value_nested_float() {
        let value = json!({"a": "{{foo.bar.baz}}"});
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": {"bar": {"baz": 1.1}}});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected = async_graphql::Value::from_json(json!({"a": 1.1})).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_render_value_arr() {
        let value = json!({"a": "{{foo.bar.baz}}"});
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": {"bar": {"baz": [1,2,3]}}});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected = async_graphql::Value::from_json(json!({"a": [1, 2, 3]})).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_render_value_arr_template() {
        let value = json!({"a": ["{{foo.bar.baz}}", "{{foo.bar.qux}}"]});
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": {"bar": {"baz": 1, "qux": 2}}});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected = async_graphql::Value::from_json(json!({"a": [1, 2]})).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_mustache_or_value_is_const() {
        let value = json!("{{foo}}");
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": "bar"});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected = async_graphql::Value::String("bar".to_owned());
        assert_eq!(result, expected);
    }

    #[test]
    fn test_mustache_arr_obj() {
        let value = json!([{"a": "{{foo.bar.baz}}"}, {"a": "{{foo.bar.qux}}"}]);
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": {"bar": {"baz": 1, "qux": 2}}});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected = async_graphql::Value::from_json(json!([{"a": 1}, {"a":2}])).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_mustache_arr_obj_arr() {
        let value = json!([{"a": [{"aa": "{{foo.bar.baz}}"}]}, {"a": [{"aa": "{{foo.bar.qux}}"}]}]);
        let value = DynamicValue::try_from(&value).unwrap();
        let ctx = json!({"foo": {"bar": {"baz": 1, "qux": 2}}});
        let result: async_graphql::Value = value.render_value(&ctx);
        let expected =
            async_graphql::Value::from_json(json!([{"a": [{"aa": 1}]}, {"a":[{"aa": 2}]}]))
                .unwrap();
        assert_eq!(result, expected);
    }
}
