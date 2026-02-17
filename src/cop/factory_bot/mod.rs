pub mod association_style;
pub mod attribute_defined_statically;
pub mod consistent_parentheses_style;
pub mod create_list;
pub mod excessive_create_list;
pub mod factory_association_with_strategy;
pub mod factory_class_name;
pub mod factory_name_style;
pub mod id_sequence;
pub mod redundant_factory_option;
pub mod syntax_methods;

use super::registry::CopRegistry;

/// Default Include patterns for FactoryBot cops that run in factory definition files.
pub const FACTORY_BOT_DEFAULT_INCLUDE: &[&str] = &[
    "**/spec/factories.rb",
    "**/spec/factories/**/*.rb",
    "**/test/factories.rb",
    "**/test/factories/**/*.rb",
    "**/features/support/factories/**/*.rb",
];

/// Default Include patterns for FactoryBot cops that also run in spec/test files.
pub const FACTORY_BOT_SPEC_INCLUDE: &[&str] = &[
    "**/*_spec.rb",
    "**/spec/**/*",
    "**/test/**/*",
    "**/features/support/factories/**/*.rb",
];

/// FactoryBot DSL method names used in specs (FactoryBot::Syntax::Methods).
pub const FACTORY_BOT_METHODS: &[&str] = &[
    "attributes_for",
    "attributes_for_list",
    "attributes_for_pair",
    "build",
    "build_list",
    "build_pair",
    "build_stubbed",
    "build_stubbed_list",
    "build_stubbed_pair",
    "create",
    "create_list",
    "create_pair",
    "generate",
    "generate_list",
    "null",
    "null_list",
    "null_pair",
];

/// FactoryBot attribute-defining methods (blocks that contain attribute DSL).
pub const ATTRIBUTE_DEFINING_METHODS: &[&[u8]] = &[
    b"factory",
    b"ignore",
    b"trait",
    b"traits_for_enum",
    b"transient",
];

/// FactoryBot reserved methods (not implicit associations).
pub const RESERVED_METHODS: &[&str] = &[
    // DEFINITION_PROXY_METHODS
    "add_attribute",
    "after",
    "association",
    "before",
    "callback",
    "ignore",
    "initialize_with",
    "sequence",
    "skip_create",
    "to_create",
    // UNPROXIED_METHODS
    "__send__",
    "__id__",
    "nil?",
    "send",
    "object_id",
    "extend",
    "instance_eval",
    "initialize",
    "block_given?",
    "raise",
    "caller",
    "method",
    // ATTRIBUTE_DEFINING_METHODS
    "factory",
    // "ignore" already listed
    "trait",
    "traits_for_enum",
    "transient",
];

/// Check if a receiver node is FactoryBot or FactoryGirl constant.
pub fn is_factory_bot_receiver(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        let name = cr.name().as_slice();
        return name == b"FactoryBot" || name == b"FactoryGirl";
    }
    if let Some(cp) = node.as_constant_path_node() {
        // ::FactoryBot
        if cp.parent().is_none() {
            if let Some(name) = cp.name() {
                let name = name.as_slice();
                return name == b"FactoryBot" || name == b"FactoryGirl";
            }
        }
    }
    false
}

/// Check if a call is a "factory_call" (receiver is FactoryBot/FactoryGirl or nil).
/// When explicit_only is true, only matches when receiver is FactoryBot/FactoryGirl.
pub fn is_factory_call(
    receiver: Option<ruby_prism::Node<'_>>,
    explicit_only: bool,
) -> bool {
    match receiver {
        Some(recv) => is_factory_bot_receiver(&recv),
        None => !explicit_only,
    }
}

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(association_style::AssociationStyle));
    registry.register(Box::new(
        attribute_defined_statically::AttributeDefinedStatically,
    ));
    registry.register(Box::new(
        consistent_parentheses_style::ConsistentParenthesesStyle,
    ));
    registry.register(Box::new(create_list::CreateList));
    registry.register(Box::new(excessive_create_list::ExcessiveCreateList));
    registry.register(Box::new(
        factory_association_with_strategy::FactoryAssociationWithStrategy,
    ));
    registry.register(Box::new(factory_class_name::FactoryClassName));
    registry.register(Box::new(factory_name_style::FactoryNameStyle));
    registry.register(Box::new(id_sequence::IdSequence));
    registry.register(Box::new(redundant_factory_option::RedundantFactoryOption));
    registry.register(Box::new(syntax_methods::SyntaxMethods));
}
