pub mod add_runtime_dependency;
pub mod attribute_assignment;
pub mod dependency_version;
pub mod deprecated_attribute_assignment;
pub mod development_dependencies;
pub mod duplicated_assignment;
pub mod ordered_dependencies;
pub mod require_mfa;
pub mod required_ruby_version;
pub mod ruby_version_globals_usage;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(
        add_runtime_dependency::AddRuntimeDependency,
    ));
    registry.register(Box::new(
        attribute_assignment::AttributeAssignment,
    ));
    registry.register(Box::new(
        dependency_version::DependencyVersion,
    ));
    registry.register(Box::new(
        deprecated_attribute_assignment::DeprecatedAttributeAssignment,
    ));
    registry.register(Box::new(
        development_dependencies::DevelopmentDependencies,
    ));
    registry.register(Box::new(
        duplicated_assignment::DuplicatedAssignment,
    ));
    registry.register(Box::new(
        ordered_dependencies::OrderedDependencies,
    ));
    registry.register(Box::new(require_mfa::RequireMfa));
    registry.register(Box::new(
        required_ruby_version::RequiredRubyVersion,
    ));
    registry.register(Box::new(
        ruby_version_globals_usage::RubyVersionGlobalsUsage,
    ));
}
