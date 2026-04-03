use crate::cop::shared::node_type::{
    BLOCK_NODE, CALL_NODE, CLASS_NODE, MODULE_NODE, SINGLETON_CLASS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// Cached corpus oracle reported FP=11, FN=2.
///
/// ### Round 1 (2026-03-10): IndentationWidth support
/// Fixed: this cop was already walking block bodies, but it still hardcoded a
/// 2-space indent for `EnforcedStyle: indent` and ignored `Layout/IndentationWidth`.
/// That produced false positives in width-4 repos and missed corresponding
/// under-indented modifiers in the same configs.
///
/// ### Round 2 (2026-03-14): Use `end` keyword column instead of opening line indentation
/// Root cause of remaining 11 FPs: the cop computed expected indentation from the
/// indentation of the line containing the opening keyword (`class`, `do`, etc.).
/// RuboCop instead measures the column offset between the access modifier and the
/// `end` keyword (or `}` for brace blocks). These differ when the opening keyword
/// is not at the start of its line (e.g., `Post = Struct.new(...) do` where `do`
/// is far right but `end` is aligned with `Post`). Also handles `Module.new do`
/// blocks where `end` is deeply indented. FN pattern: `private` at wrong column
/// relative to `end` keyword (e.g., col 4 in a class whose `end` is at col 0,
/// expecting col 2).
///
/// ### Round 3 (2026-03-31): Skip single-statement bodies
/// Root cause of remaining 2 FPs: RuboCop only checks access modifier indentation
/// when the body is `begin_type?` (i.e., 2+ statements). A body containing only
/// an access modifier and no other statements is not flagged. This matches cases
/// like `class Foo\n  protected\nend` or `module ClassMethods\n  private\nend`.
pub struct AccessModifierIndentation;

const ACCESS_MODIFIERS: &[&[u8]] = &[b"private", b"protected", b"public", b"module_function"];

fn body_statements(body: ruby_prism::Node<'_>) -> Vec<ruby_prism::Node<'_>> {
    if let Some(stmts) = body.as_statements_node() {
        stmts.body().iter().collect()
    } else {
        vec![body]
    }
}

impl Cop for AccessModifierIndentation {
    fn name(&self) -> &'static str {
        "Layout/AccessModifierIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            CALL_NODE,
            CLASS_NODE,
            MODULE_NODE,
            SINGLETON_CLASS_NODE,
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
        let style = config.get_str("EnforcedStyle", "indent");
        let indent_width = config.get_usize("IndentationWidth", 2);

        // We need a class, module, sclass, or block node that contains access modifiers.
        // Extract the body and the offset of the `end` keyword (used to compute expected
        // indentation, matching RuboCop's `column_offset_between(modifier, end_range)`).
        let (body, end_offset, container_start_offset) =
            if let Some(class_node) = node.as_class_node() {
                match class_node.body() {
                    Some(b) => (
                        b,
                        class_node.end_keyword_loc().start_offset(),
                        class_node.location().start_offset(),
                    ),
                    None => return,
                }
            } else if let Some(module_node) = node.as_module_node() {
                match module_node.body() {
                    Some(b) => (
                        b,
                        module_node.end_keyword_loc().start_offset(),
                        module_node.location().start_offset(),
                    ),
                    None => return,
                }
            } else if let Some(sclass_node) = node.as_singleton_class_node() {
                match sclass_node.body() {
                    Some(b) => (
                        b,
                        sclass_node.end_keyword_loc().start_offset(),
                        sclass_node.location().start_offset(),
                    ),
                    None => return,
                }
            } else if let Some(block_node) = node.as_block_node() {
                match block_node.body() {
                    Some(b) => (
                        b,
                        block_node.closing_loc().start_offset(),
                        block_node.location().start_offset(),
                    ),
                    None => return,
                }
            } else {
                return;
            };

        // RuboCop measures the column offset between the access modifier and the
        // `end` keyword of the enclosing scope.  For `indent` style the modifier
        // should be one `IndentationWidth` to the right of `end`; for `outdent`
        // it should be at the same column as `end`.
        let (end_line, end_col) = source.offset_to_line_col(end_offset);
        let (container_line, _) = source.offset_to_line_col(container_start_offset);

        let stmts = body_statements(body);

        // RuboCop only checks when the body is `begin_type?` (2+ statements).
        // A body with only an access modifier and no other statements is not flagged.
        if stmts.len() < 2 {
            return;
        }

        for stmt in stmts {
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            // Must be a bare access modifier (no receiver, no arguments or one argument)
            if call.receiver().is_some() {
                continue;
            }

            let method_name = call.name().as_slice();
            if !ACCESS_MODIFIERS.contains(&method_name) {
                continue;
            }

            // Check if this is a bare modifier (no args) - skip inline modifiers like `private def foo`
            if let Some(args) = call.arguments() {
                // If the argument is a def node or a symbol, it's an inline modifier
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    continue;
                }
            }

            let (mod_line, mod_col) = source.offset_to_line_col(call.location().start_offset());

            // Same line as container keyword — skip
            if mod_line == container_line {
                continue;
            }

            // Same line as end keyword — skip
            if mod_line == end_line {
                continue;
            }

            let expected_col = match style {
                "outdent" => end_col,
                _ => end_col + indent_width,
            };

            if mod_col != expected_col {
                let style_word = if style == "outdent" {
                    "Outdent"
                } else {
                    "Indent"
                };
                let modifier_name = std::str::from_utf8(method_name).unwrap_or("private");
                diagnostics.push(self.diagnostic(
                    source,
                    mod_line,
                    mod_col,
                    format!("{style_word} access modifiers like `{modifier_name}`."),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        AccessModifierIndentation,
        "cops/layout/access_modifier_indentation"
    );

    #[test]
    fn honors_indentation_width_for_block_bodies() {
        let config = CopConfig {
            options: HashMap::from([(
                "IndentationWidth".into(),
                serde_yml::Value::Number(4.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe Foo do\n    private\n    def helper; end\nend\n";
        let diags = run_cop_full_with_config(&AccessModifierIndentation, source, config);
        assert!(
            diags.is_empty(),
            "width 4 should accept a 4-space access modifier inside a block: {:?}",
            diags
        );
    }

    #[test]
    fn flags_under_indented_block_bodies_when_indentation_width_is_four() {
        let config = CopConfig {
            options: HashMap::from([(
                "IndentationWidth".into(),
                serde_yml::Value::Number(4.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe Foo do\n  private\n    def helper; end\nend\n";
        let diags = run_cop_full_with_config(&AccessModifierIndentation, source, config);
        assert_eq!(diags.len(), 1, "expected one offense, got: {:?}", diags);
        assert_eq!(diags[0].message, "Indent access modifiers like `private`.");
    }
}
