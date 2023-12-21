use std::path::Path;

use crate::blueprint::{FieldDefinition, TypeLike};
use crate::config::group_by::GroupBy;
use crate::config::{Config, Field, GraphQLOperationType, Grpc};
use crate::grpc::protobuf::{ProtobufOperation, ProtobufSet};
use crate::grpc::request_template::RequestTemplate;
use crate::json::JsonSchema;
use crate::lambda::{Expression, Lambda, Unsafe};
use crate::mustache::Mustache;
use crate::try_fold::TryFold;
use crate::valid::{Valid, ValidationError};
use crate::{config, helpers};

fn to_url(grpc: &Grpc, config: &Config) -> Valid<Mustache, String> {
  Valid::from_option(
    grpc.base_url.as_ref().or(config.upstream.base_url.as_ref()),
    "No base URL defined".to_string(),
  )
  .and_then(|base_url| {
    let mut base_url = base_url.trim_end_matches('/').to_owned();
    base_url.push('/');
    base_url.push_str(&grpc.service);
    base_url.push('/');
    base_url.push_str(&grpc.method);

    helpers::url::to_url(&base_url)
  })
}

fn to_operation(grpc: &Grpc) -> Valid<ProtobufOperation, String> {
  Valid::from(
    ProtobufSet::from_proto_file(Path::new(&grpc.proto_path)).map_err(|e| ValidationError::new(e.to_string())),
  )
  .and_then(|set| {
    Valid::from(
      set
        .find_service(&grpc.service)
        .map_err(|e| ValidationError::new(e.to_string())),
    )
  })
  .and_then(|service| {
    Valid::from(
      service
        .find_operation(&grpc.method)
        .map_err(|e| ValidationError::new(e.to_string())),
    )
  })
}

fn json_schema_from_field(config: &Config, field: &Field) -> FieldSchema {
  let field_schema = crate::blueprint::to_json_schema_for_field(field, config);
  let args_schema = crate::blueprint::to_json_schema_for_args(&field.args, config);
  FieldSchema { args: args_schema, field: field_schema }
}
pub struct FieldSchema {
  pub args: JsonSchema,
  pub field: JsonSchema,
}
fn validate_schema(field_schema: FieldSchema, operation: &ProtobufOperation, name: &str) -> Valid<(), String> {
  let input_type = &operation.input_type;
  let output_type = &operation.output_type;

  Valid::from(JsonSchema::try_from(input_type))
    .zip(Valid::from(JsonSchema::try_from(output_type)))
    .and_then(|(_input_schema, output_schema)| {
      let fields = field_schema.field;
      let _args = field_schema.args;
      fields.compare(&output_schema, name)
    })
}

pub fn update_grpc<'a>(
  operation_type: &'a GraphQLOperationType,
) -> TryFold<'a, (&'a Config, &'a Field, &'a config::Type, &'a str), FieldDefinition, String> {
  TryFold::<(&Config, &Field, &config::Type, &'a str), FieldDefinition, String>::new(
    |(config, field, _type_of, _name), b_field| {
      let Some(grpc) = &field.grpc else {
        return Valid::succeed(b_field);
      };

      to_url(grpc, config)
        .zip(to_operation(grpc))
        .zip(helpers::headers::to_headervec(&grpc.headers))
        .zip(helpers::body::to_body(grpc.body.as_deref()))
        .and_then(|(((url, operation), headers), body)| {
          let field_schema = json_schema_from_field(config, field);

          // TODO: fix validation for grpc with groupBy
          let validation = if grpc.group_by.is_empty() {
            validate_schema(field_schema, &operation, field.name()).unit()
          } else {
            Valid::succeed(())
          };

          validation.and_then(|_| {
            let req_template =
              RequestTemplate { url, headers, operation, body, operation_type: operation_type.clone() };

            if !grpc.group_by.is_empty() {
              Valid::succeed(b_field.resolver(Some(Expression::Unsafe(Unsafe::Grpc {
                req_template,
                group_by: Some(GroupBy::new(grpc.group_by.clone())),
                dl_id: None,
              }))))
            } else {
              Valid::succeed(b_field.resolver(Some(Lambda::from_grpc_request_template(req_template).expression)))
            }
          })
        })
    },
  )
}
