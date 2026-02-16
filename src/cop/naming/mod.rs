pub mod accessor_method_name;
pub mod ascii_identifiers;
pub mod binary_operator_parameter_name;
pub mod block_forwarding;
pub mod block_parameter_name;
pub mod class_and_module_camel_case;
pub mod constant_name;
pub mod file_name;
pub mod heredoc_delimiter_case;
pub mod heredoc_delimiter_naming;
pub mod memoized_instance_variable_name;
pub mod method_name;
pub mod method_parameter_name;
pub mod predicate_method;
pub mod predicate_prefix;
pub mod rescued_exceptions_variable_name;
pub mod variable_name;
pub mod variable_number;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(method_name::MethodName));
    registry.register(Box::new(variable_name::VariableName));
    registry.register(Box::new(constant_name::ConstantName));
    registry.register(Box::new(
        class_and_module_camel_case::ClassAndModuleCamelCase,
    ));
    registry.register(Box::new(accessor_method_name::AccessorMethodName));
    registry.register(Box::new(predicate_prefix::PredicatePrefix));
    registry.register(Box::new(ascii_identifiers::AsciiIdentifiers));
    registry.register(Box::new(file_name::FileName));
    registry.register(Box::new(
        binary_operator_parameter_name::BinaryOperatorParameterName,
    ));
    registry.register(Box::new(block_forwarding::BlockForwarding));
    registry.register(Box::new(block_parameter_name::BlockParameterName));
    registry.register(Box::new(
        heredoc_delimiter_case::HeredocDelimiterCase,
    ));
    registry.register(Box::new(
        heredoc_delimiter_naming::HeredocDelimiterNaming,
    ));
    registry.register(Box::new(
        memoized_instance_variable_name::MemoizedInstanceVariableName,
    ));
    registry.register(Box::new(
        method_parameter_name::MethodParameterName,
    ));
    registry.register(Box::new(
        rescued_exceptions_variable_name::RescuedExceptionsVariableName,
    ));
    registry.register(Box::new(variable_number::VariableNumber));
    registry.register(Box::new(predicate_method::PredicateMethod));
}
