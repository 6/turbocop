pub mod avoid_setup_hook;
pub mod have_http_status;
pub mod http_status;
pub mod http_status_name_consistency;
pub mod inferred_spec_type;
pub mod minitest_assertions;
pub mod negation_be_valid;
pub mod travel_around;

use super::registry::CopRegistry;

/// Default Include patterns for RSpecRails cops — only run on spec files.
pub const RSPEC_RAILS_DEFAULT_INCLUDE: &[&str] = &["**/*_spec.rb", "**/spec/**/*"];

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(avoid_setup_hook::AvoidSetupHook));
    registry.register(Box::new(have_http_status::HaveHttpStatus));
    registry.register(Box::new(http_status::HttpStatus));
    registry.register(Box::new(
        http_status_name_consistency::HttpStatusNameConsistency,
    ));
    registry.register(Box::new(inferred_spec_type::InferredSpecType));
    registry.register(Box::new(minitest_assertions::MinitestAssertions));
    registry.register(Box::new(negation_be_valid::NegationBeValid));
    registry.register(Box::new(travel_around::TravelAround));
}
