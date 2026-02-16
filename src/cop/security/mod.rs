pub mod compound_hash;
pub mod eval;
pub mod io_methods;
pub mod json_load;
pub mod marshal_load;
pub mod open;
pub mod yaml_load;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(eval::Eval));
    registry.register(Box::new(json_load::JsonLoad));
    registry.register(Box::new(yaml_load::YamlLoad));
    registry.register(Box::new(marshal_load::MarshalLoad));
    registry.register(Box::new(open::Open));
    registry.register(Box::new(io_methods::IoMethods));
    registry.register(Box::new(compound_hash::CompoundHash));
}
