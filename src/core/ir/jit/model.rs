use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use async_graphql::parser::types::{DocumentOperations, ExecutableDocument, Selection};
use async_graphql_parser::types::SelectionSet;

use super::field_index::{FieldIndex, QueryField};
use crate::core::blueprint::Blueprint;
use crate::core::ir::IR;
use crate::core::merge_right::MergeRight;

#[allow(unused)]
#[derive(Default)]
struct Counter(RefCell<usize>);
impl Counter {
    fn next(&self) -> usize {
        
        self.0.replace_with(|a| *a + 1)
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum Type {
    Named(String),
    List(Box<Type>),
    Required(Box<Type>),
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct Arg {
    pub id: ArgId,
    pub name: String,
    pub type_of: crate::core::blueprint::Type,
    pub value: Option<async_graphql_value::Value>,
    pub default_value: Option<async_graphql_value::ConstValue>,
}

#[derive(Clone)]
pub struct ArgId(usize);

impl Debug for ArgId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[allow(unused)]
impl ArgId {
    fn new(id: usize) -> Self {
        ArgId(id)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FieldId(usize);

impl Debug for FieldId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[allow(unused)]
impl FieldId {
    pub fn new(id: usize) -> Self {
        FieldId(id)
    }
}

#[derive(Clone)]
pub struct Field<A: Clone> {
    pub id: FieldId,
    pub name: String,
    pub ir: Option<IR>,
    pub type_of: crate::core::blueprint::Type,
    pub args: Vec<Arg>,
    pub refs: Option<A>,
}

const EMPTY_VEC: &Vec<Field<Children>> = &Vec::new();
impl Field<Children> {
    pub fn children(&self) -> &Vec<Field<Children>> {
        match &self.refs {
            Some(Children(children)) => children,
            _ => EMPTY_VEC,
        }
    }
}

impl Field<Parent> {
    pub fn parent(&self) -> Option<&FieldId> {
        todo!()
    }

    pub fn into_children(self, e: &ExecutionPlan) -> Field<Children> {
        let mut children = Vec::new();
        for field in e.fields.iter() {
            if let Some(id) = field.parent() {
                if *id == self.id {
                    children.push(field.to_owned().into_children(e));
                }
            }
        }

        Field {
            id: self.id,
            name: self.name,
            ir: self.ir,
            type_of: self.type_of,
            args: self.args,
            refs: Some(Children(children)),
        }
    }
}

impl<A: Debug + Clone> Debug for Field<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("Field");
        debug_struct.field("id", &self.id);
        debug_struct.field("name", &self.name);
        if self.ir.is_some() {
            debug_struct.field("ir", &"Some(..)");
        }
        debug_struct.field("type_of", &self.type_of);
        if !self.args.is_empty() {
            debug_struct.field("args", &self.args);
        }
        if self.refs.is_some() {
            debug_struct.field("refs", &self.refs);
        }
        debug_struct.finish()
    }
}

#[derive(Clone)]
#[allow(unused)]
pub struct Parent(FieldId);

impl Debug for Parent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parent({:?})", self.0)
    }
}

#[derive(Clone)]
#[allow(unused)]
pub struct Children(Vec<Field<Children>>);

#[derive(Clone, Debug)]
pub struct ExecutionPlan {
    pub fields: Vec<Field<Parent>>,
}

#[allow(unused)]
pub struct ExecutionPlanBuilder {
    index: FieldIndex,
    arg_id: Counter,
    field_id: Counter,
}

impl ExecutionPlan {
    #[allow(unused)]
    pub fn into_children(self) -> Vec<Field<Children>> {
        let this = &self.clone();
        let fields = self.fields.into_iter();

        fields.map(|f| f.into_children(this)).collect::<Vec<_>>()
    }

    #[allow(unused)]
    pub fn find_field(&self, id: FieldId) -> Option<&Field<Parent>> {
        self.fields.iter().find(|field| field.id == id)
    }
}

impl ExecutionPlanBuilder {
    #[allow(unused)]
    pub fn new(blueprint: Blueprint) -> Self {
        let blueprint_index = FieldIndex::init(&blueprint);
        Self {
            index: blueprint_index,
            arg_id: Counter::default(),
            field_id: Counter::default(),
        }
    }

    #[allow(unused)]
    pub fn build(&self, document: ExecutableDocument) -> anyhow::Result<ExecutionPlan> {
        let fields = self.create_field_set(document)?;
        Ok(ExecutionPlan { fields })
    }

    fn iter(
        &self,
        selection: SelectionSet,
        type_of: &str,
        parent: Option<Parent>,
    ) -> anyhow::Result<Vec<Field<Parent>>> {
        let mut fields = Vec::new();

        for selection in selection.items {
            if let Selection::Field(gql_field) = selection.node {
                let field_name = gql_field.node.name.node.as_str();
                let field_args = gql_field
                    .node
                    .arguments
                    .into_iter()
                    .map(|(k, v)| (k.node.as_str().to_string(), v.node))
                    .collect::<HashMap<_, _>>();

                if let Some(field_def) = self.index.get_field(type_of, field_name) {
                    let mut args = vec![];
                    for (arg_name, value) in field_args {
                        if let Some(arg) = field_def.get_arg(&arg_name) {
                            let type_of = arg.of_type.clone();
                            let id = ArgId(self.arg_id.next());
                            let arg = Arg {
                                id,
                                name: arg_name.clone(),
                                type_of,
                                value: Some(value),
                                default_value: arg
                                    .default_value
                                    .as_ref()
                                    .and_then(|v| v.to_owned().try_into().ok()),
                            };
                            args.push(arg);
                        }
                    }

                    let type_of = match field_def {
                        QueryField::Field((field_def, _)) => field_def.of_type.clone(),
                        QueryField::InputField(field_def) => field_def.of_type.clone(),
                    };

                    let cur_id = FieldId(self.field_id.next());
                    let child_fields = self.iter(
                        gql_field.node.selection_set.node.clone(),
                        type_of.name(),
                        Some(Parent(cur_id.clone())),
                    )?;
                    let field = Field {
                        id: cur_id,
                        name: field_name.to_string(),
                        ir: match field_def {
                            QueryField::Field((field_def, _)) => field_def.resolver.clone(),
                            _ => None,
                        },
                        type_of,
                        args,
                        refs: parent.clone(),
                    };

                    fields.push(field);
                    fields = fields.merge_right(child_fields);
                }
            }
        }

        Ok(fields)
    }

    fn create_field_set(&self, document: ExecutableDocument) -> anyhow::Result<Vec<Field<Parent>>> {
        let query = &self.index.get_query().to_owned();

        let mut fields = Vec::new();

        for (_, fragment) in document.fragments {
            fields = self.iter(fragment.node.selection_set.node, query, None)?;
        }

        match document.operations {
            DocumentOperations::Single(single) => {
                fields = self.iter(single.node.selection_set.node, query, None)?;
            }
            DocumentOperations::Multiple(multiple) => {
                for (_, single) in multiple {
                    fields = self.iter(single.node.selection_set.node, query, None)?;
                }
            }
        }

        Ok(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::blueprint::Blueprint;
    use crate::core::config::Config;
    use crate::core::ir::jit::model::ExecutionPlanBuilder;
    use crate::core::valid::Validator;

    const CONFIG: &str = include_str!("./fixtures/jsonplaceholder-mutation.graphql");

    fn create_query_plan(query: impl AsRef<str>) -> ExecutionPlan {
        let config = Config::from_sdl(CONFIG).to_result().unwrap();
        let blueprint = Blueprint::try_from(&config.into()).unwrap();
        let document = async_graphql::parser::parse_query(query).unwrap();

        ExecutionPlanBuilder::new(blueprint)
            .build(document)
            .unwrap()
    }

    #[tokio::test]
    async fn test_from_document() {
        let plan = create_query_plan(
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
        let plan = create_query_plan(
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
        let plan = create_query_plan(
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
        let plan = create_query_plan(
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
        let plan = create_query_plan(
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
        let plan = create_query_plan(
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
        let plan = create_query_plan(
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
        let plan = create_query_plan(
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
