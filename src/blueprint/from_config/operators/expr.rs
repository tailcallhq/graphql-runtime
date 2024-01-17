use crate::blueprint::*;
use crate::config;
use crate::config::{Config, ExprBody, Field};
use crate::lambda::{Expression, List, Logic, Math, Relation};
use crate::try_fold::TryFold;
use crate::valid::Valid;

struct CompilationContext<'a> {
  config_field: &'a config::Field,
  operation_type: &'a config::GraphQLOperationType,
  config: &'a config::Config,
}

pub fn update_expr(
  operation_type: &config::GraphQLOperationType,
) -> TryFold<'_, (&Config, &Field, &config::Type, &str), FieldDefinition, String> {
  TryFold::<(&Config, &Field, &config::Type, &str), FieldDefinition, String>::new(|(config, field, _, _), b_field| {
    let Some(expr) = &field.expr else {
      return Valid::succeed(b_field);
    };

    let context = CompilationContext { config, operation_type, config_field: field };

    compile(&context, expr.body.clone()).map(|compiled| b_field.resolver(Some(compiled)))
  })
}

///
/// Compiles a list of Exprs into a list of Expressions
///
fn compile_list(context: &CompilationContext, expr_vec: Vec<ExprBody>) -> Valid<Vec<Expression>, String> {
  Valid::from_iter(expr_vec, |value| compile(context, value))
}

///
/// Compiles a tuple of Exprs into a tuple of Expressions
///
fn compile_ab(context: &CompilationContext, ab: (ExprBody, ExprBody)) -> Valid<(Expression, Expression), String> {
  compile(context, ab.0).zip(compile(context, ab.1))
}

///
/// Compiles expr into Expression
///
fn compile(ctx: &CompilationContext, expr: ExprBody) -> Valid<Expression, String> {
  let config = ctx.config;
  let field = ctx.config_field;
  let operation_type = ctx.operation_type;
  match expr {
    // Io Expr
    ExprBody::Http(http) => compile_http(config, field, &http),
    ExprBody::Grpc(grpc) => {
      let grpc = CompileGrpc { config, field, operation_type, grpc: &grpc, validate_with_schema: false };
      compile_grpc(grpc)
    }
    ExprBody::GraphQL(gql) => compile_graphql(config, operation_type, &gql),

    // Safe Expr
    ExprBody::Const(value) => compile_const(CompileConst { config, field, value: &value, validate: false }),

    // Logic
    ExprBody::If { cond, on_true: then, on_false: els } => compile(ctx, *cond)
      .map(Box::new)
      .zip(compile(ctx, *then).map(Box::new))
      .zip(compile(ctx, *els).map(Box::new))
      .map(|((cond, then), els)| Expression::Logic(Logic::If { cond, then, els })),

    ExprBody::And(ref list) => {
      compile_list(ctx, list.clone()).map(|a| Expression::Logic(Logic::And(a)).parallel_when(expr.has_io()))
    }
    ExprBody::Or(ref list) => {
      compile_list(ctx, list.clone()).map(|a| Expression::Logic(Logic::Or(a)).parallel_when(expr.has_io()))
    }
    ExprBody::Cond(default, list) => Valid::from_iter(list, |(cond, operation)| {
      compile_ab(ctx, (*cond, *operation)).map(|(cond, operation)| (Box::new(cond), Box::new(operation)))
    })
    .and_then(|mut list| {
      compile(ctx, *default).map(|default| {
        list.push((Box::new(Expression::Literal(true.into())), Box::new(default)));
        Expression::Logic(Logic::Cond(list))
      })
    }),
    ExprBody::DefaultTo(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Logic(Logic::DefaultTo(Box::new(a), Box::new(b))))
    }
    ExprBody::IsEmpty(a) => compile(ctx, *a).map(|a| Expression::Logic(Logic::IsEmpty(Box::new(a)))),
    ExprBody::Not(a) => compile(ctx, *a).map(|a| Expression::Logic(Logic::Not(Box::new(a)))),

    // List
    ExprBody::Concat(ref values) => {
      compile_list(ctx, values.clone()).map(|a| Expression::List(List::Concat(a)).parallel_when(expr.has_io()))
    }

    // Relation
    ExprBody::Intersection(ref values) => compile_list(ctx, values.clone())
      .map(|a| Expression::Relation(Relation::Intersection(a)).parallel_when(expr.has_io())),
    ExprBody::Difference(a, b) => compile_list(ctx, a)
      .zip(compile_list(ctx, b))
      .map(|(a, b)| Expression::Relation(Relation::Difference(a, b))),
    ExprBody::Equals(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Relation(Relation::Equals(Box::new(a), Box::new(b))))
    }
    ExprBody::Gt(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Relation(Relation::Gt(Box::new(a), Box::new(b))))
    }
    ExprBody::Gte(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Relation(Relation::Gte(Box::new(a), Box::new(b))))
    }
    ExprBody::Lt(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Relation(Relation::Lt(Box::new(a), Box::new(b))))
    }
    ExprBody::Lte(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Relation(Relation::Lte(Box::new(a), Box::new(b))))
    }
    ExprBody::Max(ref list) => {
      compile_list(ctx, list.clone()).map(|a| Expression::Relation(Relation::Max(a)).parallel_when(expr.has_io()))
    }
    ExprBody::Min(ref list) => {
      compile_list(ctx, list.clone()).map(|a| Expression::Relation(Relation::Min(a)).parallel_when(expr.has_io()))
    }
    ExprBody::PathEq(a, path, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Relation(Relation::PathEq(Box::new(a), path, Box::new(b))))
    }
    ExprBody::PropEq(a, path, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Relation(Relation::PropEq(Box::new(a), path, Box::new(b))))
    }
    ExprBody::SortPath(a, path) => {
      compile(ctx, *a).map(|a| Expression::Relation(Relation::SortPath(Box::new(a), path.clone())))
    }
    ExprBody::SymmetricDifference(a, b) => compile_list(ctx, a)
      .zip(compile_list(ctx, b))
      .map(|(a, b)| Expression::Relation(Relation::SymmetricDifference(a, b))),
    ExprBody::Union(a, b) => compile_list(ctx, a)
      .zip(compile_list(ctx, b))
      .map(|(a, b)| Expression::Relation(Relation::Union(a, b))),

    // Math
    ExprBody::Mod(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Math(Math::Mod(Box::new(a), Box::new(b))))
    }
    ExprBody::Add(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Math(Math::Add(Box::new(a), Box::new(b))))
    }
    ExprBody::Dec(a) => compile(ctx, *a).map(|a| Expression::Math(Math::Dec(Box::new(a)))),
    ExprBody::Divide(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Math(Math::Divide(Box::new(a), Box::new(b))))
    }
    ExprBody::Inc(a) => compile(ctx, *a).map(|a| Expression::Math(Math::Inc(Box::new(a)))),
    ExprBody::Multiply(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Math(Math::Multiply(Box::new(a), Box::new(b))))
    }
    ExprBody::Negate(a) => compile(ctx, *a).map(|a| Expression::Math(Math::Negate(Box::new(a)))),
    ExprBody::Product(ref list) => {
      compile_list(ctx, list.clone()).map(|a| Expression::Math(Math::Product(a)).parallel_when(expr.has_io()))
    }
    ExprBody::Subtract(a, b) => {
      compile_ab(ctx, (*a, *b)).map(|(a, b)| Expression::Math(Math::Subtract(Box::new(a), Box::new(b))))
    }
    ExprBody::Sum(ref list) => {
      compile_list(ctx, list.clone()).map(|a| Expression::Math(Math::Sum(a)).parallel_when(expr.has_io()))
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use pretty_assertions::assert_eq;
  use serde_json::json;

  use super::{compile, CompilationContext};
  use crate::config::{Config, Expr, Field, GraphQLOperationType};
  use crate::http::RequestContext;
  use crate::lambda::{Concurrent, Eval, EvaluationContext, ResolverContextLike};

  #[derive(Default)]
  struct Context<'a> {
    value: Option<&'a async_graphql_value::ConstValue>,
    args: Option<&'a indexmap::IndexMap<async_graphql_value::Name, async_graphql_value::ConstValue>>,
    field: Option<async_graphql::SelectionField<'a>>,
    errors: Arc<Mutex<Vec<async_graphql::ServerError>>>,
  }

  impl<'a> ResolverContextLike<'a> for Context<'a> {
    fn value(&'a self) -> Option<&'a async_graphql_value::ConstValue> {
      self.value
    }

    fn args(&'a self) -> Option<&'a indexmap::IndexMap<async_graphql_value::Name, async_graphql_value::ConstValue>> {
      self.args
    }

    fn field(&'a self) -> Option<async_graphql::SelectionField> {
      self.field
    }

    fn add_error(&'a self, error: async_graphql::ServerError) {
      self.errors.lock().unwrap().push(error);
    }
  }

  impl Expr {
    async fn eval(expr: serde_json::Value) -> anyhow::Result<serde_json::Value> {
      let expr = serde_json::from_value::<Expr>(expr)?;
      let config = Config::default();
      let field = Field::default();
      let operation_type = GraphQLOperationType::Query;
      let context = CompilationContext { config: &config, config_field: &field, operation_type: &operation_type };
      let expression = compile(&context, expr.body.clone()).to_result()?;
      let req_ctx = RequestContext::default();
      let graphql_ctx = Context::default();
      let ctx = EvaluationContext::new(&req_ctx, &graphql_ctx);
      let value = expression.eval(&ctx, &Concurrent::default()).await?;

      Ok(serde_json::to_value(value)?)
    }
  }

  #[tokio::test]
  async fn test_is_truthy() {
    let actual = Expr::eval(json!({"body": {"inc": {"const": 1}}})).await.unwrap();
    let expected = json!(2.0);
    assert_eq!(actual, expected);
  }

  // TODO: add tests for all other expr operators
}
