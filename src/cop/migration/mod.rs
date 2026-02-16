pub mod department_name;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(department_name::DepartmentName));
}
