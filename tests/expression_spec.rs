use async_graphql::Value;
use pretty_assertions::assert_eq;
use serde_json::json;
use tailcall::blueprint::{Blueprint, DynamicValue};
use tailcall::http::RequestContext;
use tailcall::lambda::{Concurrent, EmptyResolverContext, Eval, EvaluationContext, Expression};
use tailcall::mustache::Mustache;

async fn eval(expr: &Expression) -> anyhow::Result<Value> {
    let runtime = tailcall::cli::runtime::init(&Blueprint::default());
    let req_ctx = RequestContext::new(runtime);
    let res_ctx = EmptyResolverContext {};
    let eval_ctx = EvaluationContext::new(&req_ctx, &res_ctx);
    expr.eval(eval_ctx, &Concurrent::Parallel).await
}

#[tokio::test]
async fn test_and_then() {
    let abcde = DynamicValue::try_from(&json!({"a": {"b": {"c": {"d": "e"}}}})).unwrap();
    let expr = Expression::Literal(abcde)
        .and_then(Expression::Literal(DynamicValue::Mustache(
            Mustache::parse("{{args.a}}").unwrap(),
        )))
        .and_then(Expression::Literal(DynamicValue::Mustache(
            Mustache::parse("{{args.b}}").unwrap(),
        )))
        .and_then(Expression::Literal(DynamicValue::Mustache(
            Mustache::parse("{{args.c}}").unwrap(),
        )))
        .and_then(Expression::Literal(DynamicValue::Mustache(
            Mustache::parse("{{args.d}}").unwrap(),
        )));

    let actual = eval(&expr).await.unwrap();
    let expected = Value::from_json(json!("e")).unwrap();

    assert_eq!(actual, expected);
}

#[tokio::test]
async fn test_with_args() {
    let args = Expression::Literal(
        DynamicValue::try_from(&json!({"a": {"b": {"c": {"d": "e"}}}})).unwrap(),
    );

    let expr = Expression::Literal(DynamicValue::Mustache(
        Mustache::parse("{{args.a.b.c.d}}").unwrap(),
    ))
    .with_args(args);

    let actual = eval(&expr).await.unwrap();
    let expected = Value::from_json(json!("e")).unwrap();

    assert_eq!(actual, expected);
}
