pub mod class_and_module_children;
pub mod documentation;
pub mod empty_method;
pub mod frozen_string_literal_comment;
pub mod hash_syntax;
pub mod if_unless_modifier;
pub mod lambda;
pub mod method_call_with_args_parentheses;
pub mod negated_if;
pub mod negated_while;
pub mod numeric_literals;
pub mod parentheses_around_condition;
pub mod proc;
pub mod raise_args;
pub mod redundant_return;
pub mod rescue_modifier;
pub mod rescue_standard_error;
pub mod semicolon;
pub mod signal_exception;
pub mod single_line_methods;
pub mod special_global_vars;
pub mod stabby_lambda_parentheses;
pub mod string_literals;
pub mod symbol_array;
pub mod tab;
pub mod ternary_parentheses;
pub mod trailing_comma_in_arguments;
pub mod trailing_comma_in_array_literal;
pub mod trailing_comma_in_hash_literal;
pub mod word_array;
pub mod yoda_condition;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(
        frozen_string_literal_comment::FrozenStringLiteralComment,
    ));
    registry.register(Box::new(tab::Tab));
    registry.register(Box::new(string_literals::StringLiterals));
    registry.register(Box::new(redundant_return::RedundantReturn));
    registry.register(Box::new(numeric_literals::NumericLiterals));
    registry.register(Box::new(semicolon::Semicolon));
    registry.register(Box::new(empty_method::EmptyMethod));
    registry.register(Box::new(negated_if::NegatedIf));
    registry.register(Box::new(negated_while::NegatedWhile));
    registry.register(Box::new(parentheses_around_condition::ParenthesesAroundCondition));
    registry.register(Box::new(if_unless_modifier::IfUnlessModifier));
    registry.register(Box::new(word_array::WordArray));
    registry.register(Box::new(symbol_array::SymbolArray));
    registry.register(Box::new(trailing_comma_in_arguments::TrailingCommaInArguments));
    registry.register(Box::new(trailing_comma_in_array_literal::TrailingCommaInArrayLiteral));
    registry.register(Box::new(trailing_comma_in_hash_literal::TrailingCommaInHashLiteral));
    registry.register(Box::new(class_and_module_children::ClassAndModuleChildren));
    registry.register(Box::new(ternary_parentheses::TernaryParentheses));
    registry.register(Box::new(documentation::Documentation));
    registry.register(Box::new(lambda::Lambda));
    registry.register(Box::new(self::proc::Proc));
    registry.register(Box::new(raise_args::RaiseArgs));
    registry.register(Box::new(rescue_modifier::RescueModifier));
    registry.register(Box::new(rescue_standard_error::RescueStandardError));
    registry.register(Box::new(signal_exception::SignalException));
    registry.register(Box::new(single_line_methods::SingleLineMethods));
    registry.register(Box::new(special_global_vars::SpecialGlobalVars));
    registry.register(Box::new(stabby_lambda_parentheses::StabbyLambdaParentheses));
    registry.register(Box::new(yoda_condition::YodaCondition));
    registry.register(Box::new(hash_syntax::HashSyntax));
    registry.register(Box::new(method_call_with_args_parentheses::MethodCallWithArgsParentheses));
}
