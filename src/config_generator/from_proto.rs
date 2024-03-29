#![allow(unused)]

use std::collections::{BTreeSet, HashMap};

use convert_case::{Case, Casing};
use derive_setters::Setters;
use prost_reflect::prost_types::{
    DescriptorProto, EnumDescriptorProto, FileDescriptorSet, ServiceDescriptorProto,
};
use strum_macros::Display;

use crate::blueprint::GrpcMethod;
use crate::config::{Arg, Config, Field, Grpc, Tag, Type};

pub(super) static DEFAULT_SPECTATOR: &str = "_";

/// Enum to represent the type of the descriptor
#[derive(Display, Clone)]
enum DescriptorType {
    Enum,
    Message,
    Query,
}

/// Assists in the mapping and retrieval of proto type names to custom formatted
/// strings based on the descriptor type.
#[derive(Default, Clone)]
struct Helper<T: Default + Clone> {
    /// Maps proto type names to custom formatted names.
    map: HashMap<String, String>,
    /// The current proto package name.
    package: String,
    ty: T,
}

impl<T: Default + Clone> Helper<T> {
    fn from_ty(ty: T) -> Self {
        Self { ty, ..Default::default() }
    }

    fn into_helper_config(self, config: Config) -> Helper<Config> {
        Helper { map: self.map, package: self.package, ty: config }
    }

    /// Formats a proto type name based on its `DescriptorType`.
    fn get_value(&self, name: &str, ty: DescriptorType) -> String {
        let package = self.package.replace('.', DEFAULT_SPECTATOR).to_uppercase();
        match ty {
            DescriptorType::Enum => {
                format!("{}{}{}", package, DEFAULT_SPECTATOR, name)
            }
            DescriptorType::Message => {
                format!("{}{}{}", package, DEFAULT_SPECTATOR, name)
            }
            DescriptorType::Query => format!(
                "{}{}{}",
                package.to_case(Case::Snake),
                DEFAULT_SPECTATOR,
                name.to_case(Case::Camel),
            ),
        }
    }

    /// Inserts a formatted name into the map.
    fn insert(&mut self, name: &str, ty: DescriptorType) {
        self.map.insert(
            format!("{}.{}", self.package, name),
            self.get_value(name, ty),
        );
    }
    /// Retrieves a formatted name from the map.
    fn get(&self, name: &str) -> Option<String> {
        self.map.get(&format!("{}.{}", self.package, name)).cloned()
    }
}

/// Converts proto field types to a custom format.
fn convert_ty(proto_ty: &str) -> String {
    let binding = proto_ty.to_lowercase();
    let proto_ty = binding.strip_prefix("type_").unwrap_or(proto_ty);
    match proto_ty {
        "double" | "float" => "Float",
        "int32" | "int64" | "fixed32" | "fixed64" | "uint32" | "uint64" => "Int",
        "bool" => "Boolean",
        "string" | "bytes" => "String",
        x => x,
    }
    .to_string()
}

/// Determines the output type for a service method.
fn get_output_ty(output_ty: &str) -> (String, bool) {
    // type, required
    match output_ty {
        "google.protobuf.Empty" => {
            ("String".to_string(), false) // If it's no response is expected, we
                                          // return a nullable string type
        }
        any => (any.to_string(), true), /* Setting it not null by default. There's no way to
                                         * infer this from proto file */
    }
}

/// Generates argument configurations for service methods.
fn get_arg(input_ty: &str, helper: Helper<Config>) -> Helper<Option<(String, Arg)>> {
    match input_ty {
        "google.protobuf.Empty" | "" => {
            Helper { map: helper.map, package: helper.package, ty: None }
        }
        any => {
            let key = convert_ty(any).to_case(Case::Camel);
            let val = Arg {
                type_of: helper.get(any).unwrap_or(any.to_string()),
                list: false,
                required: true,
                /* Setting it not null by default. There's no way to infer this
                 * from proto file */
                doc: None,
                modify: None,
                default_value: None,
            };

            Helper {
                map: helper.map,
                package: helper.package,
                ty: Some((key, val)),
            }
        }
    }
}

/// Retrieves or creates a Type configuration for a given proto type.
fn get_ty(name: &str, mut helper: Helper<Config>, ty: DescriptorType) -> Helper<Type> {
    helper.insert(name, ty);
    let mut ty = helper
        .ty
        .types
        .get(&helper.get(name).unwrap())
        .cloned()
        .unwrap_or_default(); // it should be
                              // safe to call
                              // unwrap here
    ty.tag = Some(Tag { id: format!("{}.{}", helper.package, name) });
    
    Helper { ty, map: helper.map, package: helper.package }
}

/// Processes proto enum types.
fn append_enums(enums: Vec<EnumDescriptorProto>, mut helper: Helper<Config>) -> Helper<Config> {
    for enum_ in enums {
        let enum_name = enum_.name();

        let mut helper_ty = get_ty(enum_name, helper.clone(), DescriptorType::Enum);
        let mut ty = helper_ty.ty.clone();
        helper = helper_ty.into_helper_config(helper.ty);

        let mut variants = enum_
            .value
            .iter()
            .map(|v| v.name().to_string())
            .collect::<BTreeSet<String>>();
        if let Some(vars) = ty.variants {
            variants.extend(vars);
        }
        ty.variants = Some(variants);
        helper.ty.types.insert(helper.get(enum_name).unwrap(), ty);
        // it should be
        // safe to call
        // unwrap here
    }
    helper
}

/// Processes proto message types.
fn append_msg_type(messages: Vec<DescriptorProto>, mut helper: Helper<Config>) -> Helper<Config> {
    if messages.is_empty() {
        return helper;
    }
    for message in messages {
        let msg_name = message.name().to_string();

        let mut helper_ty = get_ty(&msg_name, helper.clone(), DescriptorType::Message);
        let mut ty = helper_ty.ty.clone();
        helper = helper_ty.into_helper_config(helper.ty);

        helper = append_enums(message.enum_type, helper);
        helper = append_msg_type(message.nested_type, helper);

        for field in message.field {
            let field_name = field.name().to_string();
            let mut cfg_field = Field::default();

            let label = field.label().as_str_name().to_lowercase();
            cfg_field.list = label.contains("repeated");
            cfg_field.required = label.contains("required");

            if field.r#type.is_some() {
                let type_of = convert_ty(field.r#type().as_str_name());
                cfg_field.type_of = type_of.to_string();
            } else {
                // for non-primitive types
                let type_of = convert_ty(field.type_name());
                cfg_field.type_of = helper.get(&type_of).unwrap_or(type_of);
            }

            ty.fields.insert(field_name, cfg_field);
        }

        helper.ty.types.insert(helper.get(&msg_name).unwrap(), ty); // it should
                                                                    // be
                                                                    // safe to call
                                                                    // unwrap here
    }
    helper
}

/// Generates a Type configuration for service methods.
fn generate_ty(
    services: Vec<ServiceDescriptorProto>,
    mut helper: Helper<Config>,
    key: &str,
) -> Helper<Type> {
    let package = helper.package.clone();
    let mut grpc_method = GrpcMethod { package, service: "".to_string(), name: "".to_string() };
    let mut ty = helper.ty.types.get(key).cloned().unwrap_or_default();

    for service in services {
        let service_name = service.name().to_string();
        for method in &service.method {
            let method_name = method.name();

            helper.insert(method_name, DescriptorType::Query);

            let mut cfg_field = Field::default();
            let helper_arg = get_arg(method.input_type(), helper.clone());
            let arg = helper_arg.ty.clone();
            helper = helper_arg.into_helper_config(helper.ty);

            if let Some((k, v)) = arg {
                cfg_field.args.insert(k, v);
            }

            let (output_ty, required) = get_output_ty(method.output_type());
            cfg_field.type_of = helper.get(&output_ty).unwrap_or(output_ty.clone());
            cfg_field.required = required;

            grpc_method.service = service_name.clone();
            grpc_method.name = method_name.to_string();

            cfg_field.grpc = Some(Grpc {
                base_url: None,
                body: None,
                group_by: vec![],
                headers: vec![],
                method: grpc_method.to_string(),
            });
            ty.fields
                .insert(helper.get(method_name).unwrap(), cfg_field);
        }
    }
    Helper { ty, map: helper.map, package: helper.package }
}

/// Processes proto service definitions and their methods.
fn append_query_service(
    services: Vec<ServiceDescriptorProto>,
    query: &str,
    mut helper: Helper<Config>,
) -> Helper<Config> {
    if services.is_empty() {
        return helper;
    }

    let helper_ty = generate_ty(services, helper.clone(), query);
    let ty = helper_ty.ty.clone();
    helper = helper_ty.into_helper_config(helper.ty);

    if ty.ne(&Type::default()) {
        helper.ty.schema.query = Some(query.to_owned());
        helper.ty.types.insert(query.to_owned(), ty);
    }
    helper
}

/// The main entry point that builds a Config object from proto descriptor sets.
pub fn build_config(descriptor_sets: Vec<FileDescriptorSet>, query: &str) -> Config {
    let mut helper = Helper::from_ty(Config::default());

    for descriptor_set in descriptor_sets {
        for file_descriptor in descriptor_set.file {
            helper.package = file_descriptor.package().to_string();

            helper = append_enums(file_descriptor.enum_type, helper);
            helper = append_msg_type(file_descriptor.message_type, helper);
            helper = append_query_service(file_descriptor.service.clone(), query, helper);
        }
    }

    helper.ty
}

#[cfg(test)]
mod test {

    use std::path::PathBuf;

    use prost_reflect::prost_types::{FileDescriptorProto, FileDescriptorSet};

    use crate::config::Config;
    use crate::config_generator::from_proto::{build_config, DescriptorType, Helper};

    fn get_proto_file_descriptor(name: &str) -> anyhow::Result<FileDescriptorProto> {
        let mut proto_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        proto_path.push("src");
        proto_path.push("config_generator");
        proto_path.push("proto");
        proto_path.push(name);
        Ok(protox_parse::parse(
            name,
            std::fs::read_to_string(proto_path)?.as_str(),
        )?)
    }

    #[test]
    fn test_from_proto() -> anyhow::Result<()> {
        // news_enum.proto covers:
        // test for mutation
        // test for empty objects
        // test for optional type
        // test for enum
        // test for repeated fields
        // test for a type used as both input and output
        // test for two types having same name in different packages

        let mut set = FileDescriptorSet::default();

        let news = get_proto_file_descriptor("news.proto")?;
        let greetings_a = get_proto_file_descriptor("greetings_a.proto")?;
        let greetings_b = get_proto_file_descriptor("greetings_b.proto")?;

        set.file.push(news.clone());
        set.file.push(greetings_a.clone());
        set.file.push(greetings_b.clone());

        let result = build_config(vec![set], "Query").to_sdl();

        insta::assert_snapshot!(result);

        // test for 2 different sets
        let mut set = FileDescriptorSet::default();
        let mut set1 = FileDescriptorSet::default();
        let mut set2 = FileDescriptorSet::default();
        set.file.push(news);
        set1.file.push(greetings_a);
        set2.file.push(greetings_b);

        let result_sets = build_config(vec![set, set1, set2], "Query").to_sdl();

        pretty_assertions::assert_eq!(result, result_sets);

        Ok(())
    }

    #[test]
    fn test_required_types() -> anyhow::Result<()> {
        // required fields are deprecated in proto3 (https://protobuf.dev/programming-guides/dos-donts/#add-required)
        // this example uses proto2 to test the same.
        // for proto3 it's guaranteed to have a default value (https://protobuf.dev/programming-guides/proto3/#default)
        // and our implementation (https://github.com/tailcallhq/tailcall/pull/1537) supports default values by default.
        // so we do not need to explicitly mark fields as required.

        let mut set = FileDescriptorSet::default();
        let req_proto = get_proto_file_descriptor("person.proto")?;
        set.file.push(req_proto);

        let cfg = build_config(vec![set], "Query").to_sdl();
        insta::assert_snapshot!(cfg);

        Ok(())
    }
    #[test]
    fn test_get_value() {
        let mut helper: Helper<Config> =
            Helper { package: "com.example".to_string(), ..Default::default() };
        assert_eq!(
            helper.get_value("TestEnum", DescriptorType::Enum),
            "COM_EXAMPLE_TestEnum"
        );
        assert_eq!(
            helper.get_value("testMessage", DescriptorType::Message),
            "COM_EXAMPLE_testMessage"
        );
        assert_eq!(
            helper.get_value("QueryName", DescriptorType::Query),
            "com_example_queryName"
        );
    }

    #[test]
    fn test_insert_and_get() {
        let mut helper: Helper<Config> =
            Helper { package: "com.example".to_string(), ..Default::default() };
        helper.insert("TestEnum", DescriptorType::Enum);
        assert_eq!(
            helper.get("TestEnum"),
            Some("COM_EXAMPLE_TestEnum".to_string())
        );
        helper.insert("testMessage", DescriptorType::Message);
        assert_eq!(
            helper.get("testMessage"),
            Some("COM_EXAMPLE_testMessage".to_string())
        );
        assert_eq!(helper.get("NonExisting"), None);
    }
}
