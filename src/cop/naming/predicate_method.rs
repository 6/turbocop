use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct PredicateMethod;

const DEFAULT_PREFIXES: &[&str] = &["is_", "has_", "have_"];
const DEFAULT_ALLOWED: &[&str] = &["is_a?"];

impl Cop for PredicateMethod {
    fn name(&self) -> &'static str {
        "Naming/PredicateMethod"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _mode = config.get_str("Mode", "conservative");
        let allowed_methods = config.get_string_array("AllowedMethods");
        let _allowed_patterns = config.get_string_array("AllowedPatterns");
        let _allow_bang = config.get_bool("AllowBangMethods", false);
        let _name_prefix = config.get_string_array("NamePrefix");
        let _forbidden_prefixes = config.get_string_array("ForbiddenPrefixes");
        let _method_macros = config.get_string_array("MethodDefinitionMacros");
        let _wayward = config.get_string_array("WaywardPredicates");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let method_name = def_node.name().as_slice();
        let method_str = std::str::from_utf8(method_name).unwrap_or("");

        // Check allowed methods
        let allowed: Vec<String> = allowed_methods.unwrap_or_else(|| {
            DEFAULT_ALLOWED.iter().map(|s| s.to_string()).collect()
        });
        if allowed.iter().any(|a| a == method_str) {
            return Vec::new();
        }

        // Check if method name starts with a forbidden prefix
        for prefix in DEFAULT_PREFIXES {
            if method_str.starts_with(prefix) && method_str.len() > prefix.len() {
                let suffix = &method_str[prefix.len()..];
                let suggestion = format!("{suffix}?");
                let name_loc = def_node.name_loc();
                let (line, column) = source.offset_to_line_col(name_loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Rename `{method_str}` to `{suggestion}`."),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(PredicateMethod, "cops/naming/predicate_method");
}
