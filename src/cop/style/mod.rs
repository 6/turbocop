pub mod class_and_module_children;
pub mod empty_method;
pub mod frozen_string_literal_comment;
pub mod if_unless_modifier;
pub mod negated_if;
pub mod negated_while;
pub mod numeric_literals;
pub mod parentheses_around_condition;
pub mod redundant_return;
pub mod semicolon;
pub mod string_literals;
pub mod symbol_array;
pub mod tab;
pub mod ternary_parentheses;
pub mod trailing_comma_in_arguments;
pub mod trailing_comma_in_array_literal;
pub mod trailing_comma_in_hash_literal;
pub mod word_array;

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
}
