pub mod boolean_symbol;
pub mod debugger;
pub mod deprecated_class_methods;
pub mod duplicate_case_condition;
pub mod each_with_object_argument;
pub mod else_layout;
pub mod empty_conditional_body;
pub mod empty_when;
pub mod ensure_return;
pub mod float_out_of_range;
pub mod literal_as_condition;
pub mod loop_cop;
pub mod nested_method_definition;
pub mod raise_exception;
pub mod redundant_string_coercion;
pub mod suppressed_exception;
pub mod unified_integer;
pub mod uri_escape_unescape;
pub mod uri_regexp;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(debugger::Debugger));
    registry.register(Box::new(literal_as_condition::LiteralAsCondition));
    registry.register(Box::new(boolean_symbol::BooleanSymbol));
    registry.register(Box::new(deprecated_class_methods::DeprecatedClassMethods));
    registry.register(Box::new(duplicate_case_condition::DuplicateCaseCondition));
    registry.register(Box::new(each_with_object_argument::EachWithObjectArgument));
    registry.register(Box::new(else_layout::ElseLayout));
    registry.register(Box::new(empty_conditional_body::EmptyConditionalBody));
    registry.register(Box::new(empty_when::EmptyWhen));
    registry.register(Box::new(ensure_return::EnsureReturn));
    registry.register(Box::new(float_out_of_range::FloatOutOfRange));
    registry.register(Box::new(loop_cop::Loop));
    registry.register(Box::new(nested_method_definition::NestedMethodDefinition));
    registry.register(Box::new(raise_exception::RaiseException));
    registry.register(Box::new(redundant_string_coercion::RedundantStringCoercion));
    registry.register(Box::new(suppressed_exception::SuppressedException));
    registry.register(Box::new(unified_integer::UnifiedInteger));
    registry.register(Box::new(uri_escape_unescape::UriEscapeUnescape));
    registry.register(Box::new(uri_regexp::UriRegexp));
}
