pub mod debugger;
pub mod literal_as_condition;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(debugger::Debugger));
    registry.register(Box::new(literal_as_condition::LiteralAsCondition));
}
