use tailcall_valid::{Valid, Validator};

use crate::core::blueprint::{BlueprintError, FieldDefinition};
use crate::core::config;
use crate::core::config::{ConfigModule, Field, Resolver, JS};
use crate::core::ir::model::{IO, IR};
use crate::core::try_fold::TryFold;

pub struct CompileJs<'a> {
    pub js: &'a JS,
    pub script: &'a Option<String>,
}

pub fn compile_js(inputs: CompileJs) -> Valid<IR, BlueprintError> {
    let name = &inputs.js.name;
    Valid::from_option(inputs.script.as_ref(), BlueprintError::ScriptIsRequired)
        .map(|_| IR::IO(IO::Js { name: name.to_string() }))
}

pub fn update_js_field<'a>() -> TryFold<
    'a,
    (&'a ConfigModule, &'a Field, &'a config::Type, &'a str),
    FieldDefinition,
    BlueprintError,
> {
    TryFold::<(&ConfigModule, &Field, &config::Type, &str), FieldDefinition, BlueprintError>::new(
        |(module, field, _, _), b_field| {
            let Some(Resolver::Js(js)) = &field.resolver else {
                return Valid::succeed(b_field);
            };

            compile_js(CompileJs { script: &module.extensions().script, js })
                .map(|resolver| b_field.resolver(Some(resolver)))
        },
    )
}
