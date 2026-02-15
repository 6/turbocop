use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DynamicFindBy;

impl Cop for DynamicFindBy {
    fn name(&self) -> &'static str {
        "Rails/DynamicFindBy"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // AllowedMethods (Whitelist is deprecated alias)
        let allowed = config.get_string_array("AllowedMethods");
        let whitelist = config.get_string_array("Whitelist");
        let allowed_receivers = config.get_string_array("AllowedReceivers");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        let name = call.name().as_slice();
        if !name.starts_with(b"find_by_") {
            return Vec::new();
        }
        if call.receiver().is_none() {
            return Vec::new();
        }

        // Skip if method is in AllowedMethods or Whitelist (deprecated alias)
        let name_str = std::str::from_utf8(name).unwrap_or("");
        if let Some(ref list) = allowed {
            if list.iter().any(|m| m == name_str) {
                return Vec::new();
            }
        }
        if let Some(ref list) = whitelist {
            if list.iter().any(|m| m == name_str) {
                return Vec::new();
            }
        }

        // Skip if receiver is in AllowedReceivers
        if let Some(ref receivers) = allowed_receivers {
            if let Some(recv) = call.receiver() {
                let recv_bytes = recv.location().as_slice();
                let recv_str = std::str::from_utf8(recv_bytes).unwrap_or("");
                if receivers.iter().any(|r| r == recv_str) {
                    return Vec::new();
                }
            }
        }

        let attr = &name[b"find_by_".len()..];
        let attr_str = std::str::from_utf8(attr).unwrap_or("...");
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let msg = format!(
            "Use `find_by({attr_str}: ...)` instead of `{}`.",
            std::str::from_utf8(name).unwrap_or("find_by_...")
        );
        vec![self.diagnostic(source, line, column, msg)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DynamicFindBy, "cops/rails/dynamic_find_by");

    #[test]
    fn whitelist_suppresses_offense() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "Whitelist".to_string(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("find_by_name".to_string()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"User.find_by_name('foo')\n";
        let diags = run_cop_full_with_config(&DynamicFindBy, source, config);
        assert!(diags.is_empty(), "Whitelist should suppress offense for find_by_name");
    }
}
