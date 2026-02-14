pub mod abc_size;
pub mod block_length;
pub mod class_length;
pub mod cyclomatic_complexity;
pub mod method_length;
pub mod module_length;
pub mod parameter_lists;
pub mod perceived_complexity;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(method_length::MethodLength));
    registry.register(Box::new(class_length::ClassLength));
    registry.register(Box::new(module_length::ModuleLength));
    registry.register(Box::new(block_length::BlockLength));
    registry.register(Box::new(parameter_lists::ParameterLists));
    registry.register(Box::new(abc_size::AbcSize));
    registry.register(Box::new(cyclomatic_complexity::CyclomaticComplexity));
    registry.register(Box::new(perceived_complexity::PerceivedComplexity));
}
