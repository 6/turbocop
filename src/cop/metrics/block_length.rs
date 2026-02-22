use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::util::{collect_foldable_ranges, count_body_lines, count_body_lines_ex};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct BlockLength;

impl Cop for BlockLength {
    fn name(&self) -> &'static str {
        "Metrics/BlockLength"
    }

    fn default_exclude(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // We check CallNode (not BlockNode) so we can read the method name
        // for AllowedMethods/AllowedPatterns filtering.
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let block_node = match call_node.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return, // Lambda literal etc
            },
            None => return,
        };

        // RuboCop skips class constructor blocks (Struct.new, Class.new, etc.)
        if is_class_constructor(&call_node) {
            return;
        }

        let max = config.get_usize("Max", 25);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        // AllowedMethods / AllowedPatterns: skip blocks on matching method calls
        let method_name = std::str::from_utf8(call_node.name().as_slice()).unwrap_or("");
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        if let Some(allowed) = &allowed_methods {
            if allowed.iter().any(|m| m == method_name) {
                return;
            }
        }
        if let Some(patterns) = &allowed_patterns {
            for pat in patterns {
                if let Ok(re) = regex::Regex::new(pat) {
                    if re.is_match(method_name) {
                        return;
                    }
                }
            }
        }

        let start_offset = block_node.opening_loc().start_offset();
        let end_offset = block_node.closing_loc().start_offset();
        let count = if let Some(cao) = &count_as_one {
            if !cao.is_empty() {
                if let Some(body) = block_node.body() {
                    let foldable = collect_foldable_ranges(source, &body, cao);
                    count_body_lines_ex(source, start_offset, end_offset, count_comments, &foldable)
                } else {
                    0
                }
            } else {
                count_body_lines(source, start_offset, end_offset, count_comments)
            }
        } else {
            count_body_lines(source, start_offset, end_offset, count_comments)
        };

        if count > max {
            // Use call_node location (not block opening) to match RuboCop's
            // offense position which spans the full expression in Parser AST.
            let (line, column) = source.offset_to_line_col(call_node.location().start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Block has too many lines. [{count}/{max}]"),
            ));
        }
    }
}

/// Check if a call is a class constructor like `Struct.new`, `Class.new`, `Module.new`, etc.
/// RuboCop's Metrics/BlockLength does not count these blocks.
fn is_class_constructor(call: &ruby_prism::CallNode<'_>) -> bool {
    if call.name().as_slice() != b"new" {
        return false;
    }
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };
    // Check for simple constant receiver (Struct, Class, Module, etc.)
    if let Some(cr) = recv.as_constant_read_node() {
        let name = cr.name().as_slice();
        return matches!(name, b"Struct" | b"Class" | b"Module");
    }
    // Check for constant path (e.g., ::Struct.new)
    if let Some(cp) = recv.as_constant_path_node() {
        if let Some(name_node) = cp.name() {
            let name = name_node.as_slice();
            return matches!(name, b"Struct" | b"Class" | b"Module");
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BlockLength, "cops/metrics/block_length");

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 4 body lines exceeds Max:3
        let source = b"items.each do |x|\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:3 on 4-line block");
        assert!(diags[0].message.contains("[4/3]"));
    }

    #[test]
    fn config_count_as_one_array() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                (
                    "CountAsOne".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("array".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // Body: a, b, [\n1,\n2\n] = 2 + 1 folded = 3 lines
        let source = b"items.each do |x|\n  a = 1\n  b = 2\n  arr = [\n    1,\n    2\n  ]\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(
            diags.is_empty(),
            "Should not fire when array is folded (3/3)"
        );
    }

    #[test]
    fn allowed_methods_refine() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                (
                    "AllowedMethods".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("refine".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // refine block with 4 lines should NOT fire because refine is allowed
        let source =
            b"refine String do\n  def a; end\n  def b; end\n  def c; end\n  def d; end\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(
            diags.is_empty(),
            "Should not fire on allowed method 'refine'"
        );
    }
}
