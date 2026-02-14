pub mod frozen_string_literal_comment;
pub mod tab;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(
        frozen_string_literal_comment::FrozenStringLiteralComment,
    ));
    registry.register(Box::new(tab::Tab));
}
