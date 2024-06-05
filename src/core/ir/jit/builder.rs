use std::collections::HashMap;

use async_graphql::parser::types::{DocumentOperations, ExecutableDocument, Selection};
use async_graphql_parser::types::SelectionSet;

use super::field_index::{FieldIndex, QueryField};
use super::model::*;
use crate::core::blueprint::Blueprint;
use crate::core::counter::Counter;
use crate::core::merge_right::MergeRight;
use crate::core::valid::Validator;

pub struct ExecutionPlanBuilder {
    index: FieldIndex,
    arg_id: Counter,
    field_id: Counter,
    document: ExecutableDocument,
}

impl ExecutionPlanBuilder {
    #[allow(unused)]
    pub fn new(blueprint: Blueprint, document: ExecutableDocument) -> Self {
        let blueprint_index = FieldIndex::init(&blueprint);
        Self {
            document,
            index: blueprint_index,
            arg_id: Counter::default(),
            field_id: Counter::default(),
        }
    }

    fn iter(
        &self,
        selection: &SelectionSet,
        type_of: &str,
        refs: Option<Parent>,
    ) -> Vec<Field<Parent>> {
        let mut fields = vec![];
        for selection in &selection.items {
            if let Selection::Field(gql_field) = &selection.node {
                let field_name = gql_field.node.name.node.as_str();
                let field_args = gql_field
                    .node
                    .arguments
                    .iter()
                    .map(|(k, v)| (k.node.as_str().to_string(), v.node.to_owned()))
                    .collect::<HashMap<_, _>>();

                if let Some(field_def) = self.index.get_field(type_of, field_name) {
                    let mut args = vec![];
                    for (arg_name, value) in field_args {
                        if let Some(arg) = field_def.get_arg(&arg_name) {
                            let type_of = arg.of_type.clone();
                            let id = ArgId::new(self.arg_id.next());
                            let name = arg_name.clone();
                            let default_value = arg
                                .default_value
                                .as_ref()
                                .and_then(|v| v.to_owned().try_into().ok());
                            args.push(Arg { id, name, type_of, value: Some(value), default_value });
                        }
                    }

                    let type_of = match field_def {
                        QueryField::Field((field_def, _)) => field_def.of_type.clone(),
                        QueryField::InputField(field_def) => field_def.of_type.clone(),
                    };

                    let id = FieldId::new(self.field_id.next());
                    let child_fields = self.iter(
                        &gql_field.node.selection_set.node,
                        type_of.name(),
                        Some(Parent::new(id.clone())),
                    );
                    let name = field_name.to_owned();
                    let ir = match field_def {
                        QueryField::Field((field_def, _)) => field_def.resolver.clone(),
                        _ => None,
                    };
                    fields.push(Field { id, name, ir, type_of, args, refs: refs.clone() });
                    fields = fields.merge_right(child_fields);
                }
            }
        }

        fields
    }

    pub fn build(&self) -> ExecutionPlan {
        let query = &self.index.get_query().to_owned();

        let mut fields = Vec::new();

        for fragment in self.document.fragments.values() {
            fields = self.iter(&fragment.node.selection_set.node, query, None);
        }

        match &self.document.operations {
            DocumentOperations::Single(single) => {
                fields = self.iter(&single.node.selection_set.node, query, None);
            }
            DocumentOperations::Multiple(multiple) => {
                for single in multiple.values() {
                    fields = self.iter(&single.node.selection_set.node, query, None);
                }
            }
        }

        ExecutionPlan { fields }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::blueprint::Blueprint;
    use crate::core::config::Config;
    use crate::core::ir::jit::builder::ExecutionPlanBuilder;
    use crate::core::valid::Validator;

    const CONFIG: &str = include_str!("./fixtures/jsonplaceholder-mutation.graphql");

    fn plan(query: impl AsRef<str>) -> ExecutionPlan {
        let config = Config::from_sdl(CONFIG).to_result().unwrap();
        let blueprint = Blueprint::try_from(&config.into()).unwrap();
        let document = async_graphql::parser::parse_query(query).unwrap();

        ExecutionPlanBuilder::new(blueprint, document).build()
    }

    #[tokio::test]
    async fn test_from_document() {
        let plan = plan(
            r#"
            query {
                posts { user { id name } }
            }
        "#,
        );
        insta::assert_debug_snapshot!(plan);
    }

    #[test]
    fn test_simple_query() {
        let plan = plan(
            r#"
            query {
                posts { user { id } }
            }
        "#,
        );
        insta::assert_debug_snapshot!(plan);
    }

    #[test]
    fn test_simple_mutation() {
        let plan = plan(
            r#"
            mutation {
              createUser(user: {
                id: 101,
                name: "Tailcall",
                email: "tailcall@tailcall.run",
                phone: "2345234234",
                username: "tailcall",
                website: "tailcall.run"
              }) {
                id
                name
                email
                phone
                website
                username
              }
            }
        "#,
        );
        insta::assert_debug_snapshot!(plan);
    }

    #[test]
    fn test_fragments() {
        let plan = plan(
            r#"
            fragment UserPII on User {
              name
              email
              phone
            }

            query {
              user(id:1) {
                ...UserPII
              }
            }
        "#,
        );
        insta::assert_debug_snapshot!(plan);
    }

    #[test]
    fn test_multiple_operations() {
        let plan = plan(
            r#"
            query {
              user(id:1) {
                id
                username
              }
              posts {
                id
                title
              }
            }
        "#,
        );
        insta::assert_debug_snapshot!(plan);
    }

    #[test]
    fn test_variables() {
        let plan = plan(
            r#"
            query user($id: Int!) {
              user(id: $id) {
                id
                name
              }
            }
        "#,
        );
        insta::assert_debug_snapshot!(plan);
    }

    #[test]
    fn test_unions() {
        let plan = plan(
            r#"
            query {
              getUserIdOrEmail(id:1) {
                ...on UserId {
                  id
                }
                ...on UserEmail {
                  email
                }
              }
            }
        "#,
        );
        insta::assert_debug_snapshot!(plan);
    }

    #[test]
    fn test_default_value() {
        let plan = plan(
            r#"
            mutation {
              createPost(post:{
                userId:123,
                title:"tailcall",
                body:"tailcall test"
              }) {
                id
              }
            }
        "#,
        );
        insta::assert_debug_snapshot!(plan);
    }
}
