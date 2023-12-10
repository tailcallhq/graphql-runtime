use std::borrow::Cow;
use std::collections::BTreeMap;

use async_graphql::{Name, Value};
use hyper::HeaderMap;
use indexmap::IndexMap;

use tailcall::lambda::{EvaluationContext, ResolverContextLike};
use tailcall::path_string::PathString;

// Use lazy_static to initialize static variables once
lazy_static::lazy_static! {
    pub static ref TEST_VALUES: Value = {
        let mut root = IndexMap::new();
        let mut nested = IndexMap::new();

        nested.insert(Name::new("existing"), Value::String("nested-test".to_owned()));

        root.insert(Name::new("root"), Value::String("root-test".to_owned()));
        root.insert(Name::new("nested"), Value::Object(nested));

        Value::Object(root)
    };

    pub static ref TEST_ARGS: IndexMap<Name, Value> = {
        let mut root = IndexMap::new();
        let mut nested = IndexMap::new();

        nested.insert(Name::new("existing"), Value::String("nested-test".to_owned()));

        root.insert(Name::new("root"), Value::String("root-test".to_owned()));
        root.insert(Name::new("nested"), Value::Object(nested));

        root
    };

    pub static ref TEST_HEADERS: HeaderMap = {
        let mut map = HeaderMap::new();

        map.insert("x-existing", "header".parse().unwrap());

        map
    };

    pub static ref TEST_VARS: BTreeMap<String, String> = {
        let mut map = BTreeMap::new();

        map.insert("existing".to_owned(), "var".to_owned());

        map
    };
}

// Define a mock GraphQL context
pub struct MockGraphqlContext;

// Implement ResolverContextLike trait for MockGraphqlContext
impl<'a> ResolverContextLike<'a> for MockGraphqlContext {
  fn value(&'a self) -> Option<&'a Value> {
    Some(&TEST_VALUES)
  }

  fn args(&'a self) -> Option<&'a IndexMap<Name, Value>> {
    Some(&TEST_ARGS)
  }
}

// Function to assert test values using EvaluationContext
pub fn assert_test(eval_ctx: &EvaluationContext<'_, MockGraphqlContext>) {
  // value assertions
  assert_eq!(
    eval_ctx.path_string(&["value", "root"]),
    Some(Cow::Borrowed("root-test"))
  );
  assert_eq!(
    eval_ctx.path_string(&["value", "nested", "existing"]),
    Some(Cow::Borrowed("nested-test"))
  );
  assert_eq!(eval_ctx.path_string(&["value", "missing"]), None);
  assert_eq!(eval_ctx.path_string(&["value", "nested", "missing"]), None);

  // args assertions
  assert_eq!(
    eval_ctx.path_string(&["args", "root"]),
    Some(Cow::Borrowed("root-test"))
  );
  assert_eq!(
    eval_ctx.path_string(&["args", "nested", "existing"]),
    Some(Cow::Borrowed("nested-test"))
  );
  assert_eq!(eval_ctx.path_string(&["args", "missing"]), None);
  assert_eq!(eval_ctx.path_string(&["args", "nested", "missing"]), None);

  // headers assertions
  assert_eq!(
    eval_ctx.path_string(&["headers", "x-existing"]),
    Some(Cow::Borrowed("header"))
  );
  assert_eq!(eval_ctx.path_string(&["headers", "x-missing"]), None);

  // vars assertions
  assert_eq!(eval_ctx.path_string(&["vars", "existing"]), Some(Cow::Borrowed("var")));
  assert_eq!(eval_ctx.path_string(&["vars", "missing"]), None);
}
