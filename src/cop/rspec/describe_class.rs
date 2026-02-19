use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, KEYWORD_HASH_NODE, PROGRAM_NODE, STRING_NODE, SYMBOL_NODE};

/// RSpec/DescribeClass: The first argument to top-level describe should be
/// the class or module being tested.
pub struct DescribeClass;

impl Cop for DescribeClass {
    fn name(&self) -> &'static str {
        "RSpec/DescribeClass"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, KEYWORD_HASH_NODE, PROGRAM_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let stmts = program.statements();
        let mut diagnostics = Vec::new();

        for stmt in stmts.body().iter() {
            check_top_level_describe(self, source, &stmt, &mut diagnostics, config);
        }

        diagnostics
    }
}

fn check_top_level_describe(
    cop: &DescribeClass,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    config: &CopConfig,
) {
    // Config: IgnoredMetadata — metadata keys that make describe acceptable
    let ignored_metadata = config.get_string_array("IgnoredMetadata");
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return,
    };

    let name = call.name().as_slice();
    if name != b"describe" {
        return;
    }

    // Must be receiverless or RSpec.describe / ::RSpec.describe
    if let Some(recv) = call.receiver() {
        let is_rspec = if let Some(cr) = recv.as_constant_read_node() {
            cr.name().as_slice() == b"RSpec"
        } else if let Some(cp) = recv.as_constant_path_node() {
            cp.name().map_or(false, |n| n.as_slice() == b"RSpec") && cp.parent().is_none()
        } else {
            false
        };
        if !is_rspec {
            return;
        }
    }

    // Check if inside shared_examples/shared_context - skip if so
    // (this is handled by the fact that we only check top-level statements)

    let args = match call.arguments() {
        Some(a) => a,
        None => return, // No arguments = empty describe, OK
    };

    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return;
    }

    let first_arg = &arg_list[0];

    // If first arg is a constant or constant path, it's fine
    if first_arg.as_constant_read_node().is_some() || first_arg.as_constant_path_node().is_some() {
        return;
    }

    // If first arg is a string, check if it looks like a class/module name
    if let Some(s) = first_arg.as_string_node() {
        let value = s.unescaped();
        if looks_like_constant(value) {
            return; // String that looks like a constant name is OK
        }
    }

    // Check for `type:` metadata - ignore if has type
    if has_type_metadata(&arg_list) {
        return;
    }

    // Check for IgnoredMetadata keys
    if let Some(ref keys) = ignored_metadata {
        if has_ignored_metadata(&arg_list, keys) {
            return;
        }
    }

    // Flag the first argument
    let loc = first_arg.location();
    let (line, col) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        col,
        "The first argument to describe should be the class or module being tested.".to_string(),
    ));
}

/// Check if a string value looks like a Ruby constant name (starts uppercase, no spaces).
fn looks_like_constant(value: &[u8]) -> bool {
    if value.is_empty() {
        return false;
    }
    // Must start with uppercase letter or ::
    let start = if value.starts_with(b"::") {
        &value[2..]
    } else {
        value
    };
    if start.is_empty() || !start[0].is_ascii_uppercase() {
        return false;
    }
    // All characters must be alphanumeric, underscore, or ::
    let mut i = 0;
    while i < value.len() {
        let b = value[i];
        if b.is_ascii_alphanumeric() || b == b'_' {
            i += 1;
        } else if b == b':' && i + 1 < value.len() && value[i + 1] == b':' {
            i += 2;
        } else {
            return false;
        }
    }
    true
}

fn has_ignored_metadata(args: &[ruby_prism::Node<'_>], keys: &[String]) -> bool {
    for arg in args {
        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        let key_name = sym.unescaped();
                        for k in keys {
                            if key_name == k.as_bytes() {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn has_type_metadata(args: &[ruby_prism::Node<'_>]) -> bool {
    for arg in args {
        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if sym.unescaped() == b"type" {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(DescribeClass, "cops/rspec/describe_class");

    #[test]
    fn ignored_metadata_skips_describe_with_matching_key() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoredMetadata".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("feature".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe 'some feature', feature: true do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&DescribeClass, source, config);
        assert!(diags.is_empty(), "Should skip when IgnoredMetadata key is present");
    }

    #[test]
    fn ignored_metadata_still_flags_without_matching_key() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoredMetadata".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("feature".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe 'some feature' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&DescribeClass, source, config);
        assert_eq!(diags.len(), 1);
    }
}
