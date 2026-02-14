pub mod frozen_string_literal_comment;
pub mod numeric_literals;
pub mod redundant_return;
pub mod semicolon;
pub mod string_literals;
pub mod tab;

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
}
