pub mod accessor_method_name;
pub mod ascii_identifiers;
pub mod class_and_module_camel_case;
pub mod constant_name;
pub mod file_name;
pub mod method_name;
pub mod predicate_name;
pub mod variable_name;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(method_name::MethodName));
    registry.register(Box::new(variable_name::VariableName));
    registry.register(Box::new(constant_name::ConstantName));
    registry.register(Box::new(
        class_and_module_camel_case::ClassAndModuleCamelCase,
    ));
    registry.register(Box::new(accessor_method_name::AccessorMethodName));
    registry.register(Box::new(predicate_name::PredicateName));
    registry.register(Box::new(ascii_identifiers::AsciiIdentifiers));
    registry.register(Box::new(file_name::FileName));
}
