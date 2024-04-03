use core::future::Future;
use std::fmt::{Debug, Display};
use std::pin::Pin;

use anyhow::{anyhow, Result};
use async_graphql_value::ConstValue;
use thiserror::Error;

use super::{Concurrent, Eval, EvaluationContext, ResolverContextLike, IO};
use crate::blueprint::DynamicValue;
use crate::json::JsonLike;
use crate::lambda::cache::Cache;
use crate::serde_value_ext::ValueExt;

#[derive(Clone, Debug)]
pub enum Expression {
    Context(Context),
    Literal(DynamicValue),
    EqualTo(Box<Expression>, Box<Expression>),
    IO(IO),
    Cache(Cache),
    Input(Box<Expression>, Vec<String>),
    Concurrency(Concurrent, Box<Expression>),
    Protect(Box<Expression>),
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Context(_) => write!(f, "Context"),
            Expression::Literal(_) => write!(f, "Literal"),
            Expression::EqualTo(_, _) => write!(f, "EqualTo"),
            Expression::IO(io) => write!(f, "{io}"),
            Expression::Cache(_) => write!(f, "Cache"),
            Expression::Input(_, _) => write!(f, "Input"),
            Expression::Concurrency(conc, _) => write!(f, "Concurrency({conc})"),
            Expression::Protect(expr) => write!(f, "Protected({expr})"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Context {
    Value,
    Path(Vec<String>),
    PushArgs {
        expr: Box<Expression>,
        and_then: Box<Expression>,
    },
    PushValue {
        expr: Box<Expression>,
        and_then: Box<Expression>,
    },
}

#[derive(Debug, Error)]
pub enum EvaluationError {
    #[error("IOException: {0}")]
    IOException(String),

    #[error("APIValidationError: {0:?}")]
    APIValidationError(Vec<String>),

    #[error("ExprEvalError: {0:?}")]
    ExprEvalError(String),
}

impl<'a> From<crate::valid::ValidationError<&'a str>> for EvaluationError {
    fn from(_value: crate::valid::ValidationError<&'a str>) -> Self {
        EvaluationError::APIValidationError(
            _value
                .as_vec()
                .iter()
                .map(|e| e.message.to_owned())
                .collect(),
        )
    }
}

impl Expression {
    pub fn concurrency(self, conc: Concurrent) -> Self {
        Expression::Concurrency(conc, Box::new(self))
    }

    pub fn in_parallel(self) -> Self {
        self.concurrency(Concurrent::Parallel)
    }

    pub fn parallel_when(self, cond: bool) -> Self {
        if cond {
            self.concurrency(Concurrent::Parallel)
        } else {
            self
        }
    }

    pub fn in_sequence(self) -> Self {
        self.concurrency(Concurrent::Sequential)
    }

    pub fn and_then(self, next: Self) -> Self {
        Expression::Context(Context::PushArgs { expr: Box::new(self), and_then: Box::new(next) })
    }

    pub fn with_args(self, args: Expression) -> Self {
        Expression::Context(Context::PushArgs { expr: Box::new(args), and_then: Box::new(self) })
    }
}

impl Eval for Expression {
    #[tracing::instrument(skip_all, fields(otel.name = %self), err)]
    fn eval<'a, Ctx: ResolverContextLike<'a> + Sync + Send>(
        &'a self,
        ctx: EvaluationContext<'a, Ctx>,
        conc: &'a Concurrent,
    ) -> Pin<Box<dyn Future<Output = Result<ConstValue>> + 'a + Send>> {
        Box::pin(async move {
            match self {
                Expression::Concurrency(conc, expr) => Ok(expr.eval(ctx, conc).await?),
                Expression::Context(op) => match op {
                    Context::Value => {
                        Ok(ctx.value().cloned().unwrap_or(async_graphql::Value::Null))
                    }
                    Context::Path(path) => Ok(ctx
                        .path_value(path)
                        .map(|a| a.into_owned())
                        .unwrap_or(async_graphql::Value::Null)),
                    Context::PushArgs { expr, and_then } => {
                        let args = expr.eval(ctx.clone(), conc).await?;
                        let ctx = ctx.with_args(args).clone();
                        and_then.eval(ctx, conc).await
                    }
                    Context::PushValue { expr, and_then } => {
                        let value = expr.eval(ctx.clone(), conc).await?;
                        let ctx = ctx.with_value(value);
                        and_then.eval(ctx, conc).await
                    }
                },
                Expression::Input(input, path) => {
                    let inp = &input.eval(ctx, conc).await?;
                    Ok(inp
                        .get_path(path)
                        .unwrap_or(&async_graphql::Value::Null)
                        .clone())
                }
                Expression::Literal(value) => value.render_value(&ctx),
                Expression::EqualTo(left, right) => Ok(async_graphql::Value::from(
                    left.eval(ctx.clone(), conc).await? == right.eval(ctx, conc).await?,
                )),
                Expression::Protect(expr) => {
                    ctx.request_ctx
                        .auth_ctx
                        .validate(ctx.request_ctx)
                        .await
                        .to_result()
                        .map_err(|e| anyhow!("Authentication Failure: {}", e.to_string()))?;
                    expr.eval(ctx, conc).await
                }
                Expression::IO(operation) => operation.eval(ctx, conc).await,
                Expression::Cache(cached) => cached.eval(ctx, conc).await,
            }
        })
    }
}
