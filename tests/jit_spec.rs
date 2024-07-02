#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use async_graphql::Value;
    use async_graphql_value::ConstValue;
    use tailcall::core::app_context::AppContext;
    use tailcall::core::blueprint::Blueprint;
    use tailcall::core::config::{Config, ConfigModule};
    use tailcall::core::jit::{ConstValueExecutor, Request};
    use tailcall::core::rest::EndpointSet;
    use tailcall::core::valid::Validator;

    async fn new_executor(request: &Request<Value>) -> anyhow::Result<ConstValueExecutor> {
        let sdl = tokio::fs::read_to_string(tailcall_fixtures::configs::USER_POSTS).await?;
        let config = Config::from_sdl(&sdl).to_result()?;
        let blueprint = Blueprint::try_from(&ConfigModule::from(config))?;
        let runtime = tailcall::cli::runtime::init(&blueprint);
        let app_ctx = Arc::new(AppContext::new(blueprint, runtime, EndpointSet::default()));

        Ok(ConstValueExecutor::new(request, app_ctx).unwrap())
    }

    #[tokio::test]
    async fn test_executor() {
        //  NOTE: This test makes a real HTTP call
        let request = Request::new("query {posts {id title}}");
        let executor = new_executor(&request).await.unwrap();
        let response = executor.execute(request).await;
        let data = response.data;

        insta::assert_json_snapshot!(data);
    }

    #[tokio::test]
    async fn test_executor_nested() {
        //  NOTE: This test makes a real HTTP call
        let request = Request::new("query {posts {title userId user {id name email} }}");
        let executor = new_executor(&request).await.unwrap();
        let response = executor.execute(request).await;
        let data = response.data;

        insta::assert_json_snapshot!(data);
    }

    #[tokio::test]
    async fn test_executor_arguments() {
        //  NOTE: This test makes a real HTTP call
        let request = Request::new("query {user(id: 1) {id}}");
        let executor = new_executor(&request).await.unwrap();
        let response = executor.execute(request).await;
        let data = response.data;

        insta::assert_json_snapshot!(data);
    }

    #[tokio::test]
    async fn test_executor_arguments_default_value() {
        //  NOTE: This test makes a real HTTP call
        let request = Request::new("query {user2 {id name}}");
        let executor = new_executor(&request).await.unwrap();
        let response = executor.execute(request).await;
        let data = response.data;

        insta::assert_json_snapshot!(data);
    }

    #[tokio::test]
    async fn test_executor_variables() {
        //  NOTE: This test makes a real HTTP call
        let query = r#"
            query user($id: Int!) {
              user(id: $id) {
                id
                name
              }
            }
        "#;
        let request = Request::new(query);
        let request = request.variables(HashMap::from([("id".into(), ConstValue::from(1))]));
        let executor = new_executor(&request).await.unwrap();
        let response = executor.execute(request).await;
        let data = response.data;

        insta::assert_json_snapshot!(data);
    }
}
