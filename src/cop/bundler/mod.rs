pub mod duplicated_gem;
pub mod duplicated_group;
pub mod gem_comment;
pub mod gem_filename;
pub mod gem_version;
pub mod insecure_protocol_source;
pub mod ordered_gems;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(duplicated_gem::DuplicatedGem));
    registry.register(Box::new(duplicated_group::DuplicatedGroup));
    registry.register(Box::new(gem_comment::GemComment));
    registry.register(Box::new(gem_filename::GemFilename));
    registry.register(Box::new(gem_version::GemVersion));
    registry.register(Box::new(insecure_protocol_source::InsecureProtocolSource));
    registry.register(Box::new(ordered_gems::OrderedGems));
}

/// Extract the gem name from a line like `gem 'foo'` or `gem "foo"` or `gem('foo')`.
/// Returns Some(gem_name) if the line is a gem declaration, None otherwise.
pub fn extract_gem_name(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with("gem ") && !trimmed.starts_with("gem(") {
        return None;
    }
    // Find first quote
    let quote_start = trimmed.find(['\'', '"'])?;
    let rest = &trimmed[quote_start + 1..];
    let quote_char = trimmed.as_bytes()[quote_start];
    let quote_end = rest.find(|c: char| c as u8 == quote_char)?;
    Some(&rest[..quote_end])
}
