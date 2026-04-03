use crate::cop::shared::node_type::{
    CLASS_NODE, CONSTANT_OR_WRITE_NODE, CONSTANT_PATH_OR_WRITE_NODE, CONSTANT_PATH_WRITE_NODE,
    CONSTANT_WRITE_NODE, MODULE_NODE, MULTI_WRITE_NODE, STATEMENTS_NODE,
};
use crate::cop::shared::util::{
    collect_foldable_ranges, count_body_lines_ex, count_body_lines_full, inner_classlike_ranges,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-04)
///
/// Corpus oracle reported FP=1, FN=0.
///
/// FP=1 came from a short-form disable directive
/// (`# rubocop:disable ModuleLength`) that RuboCop resolves to this cop. The
/// prior directive matcher only handled fully-qualified names.
///
/// Fix applied in this batch: short cop name resolution in
/// `parse::directives` (framework-level), which this cop now benefits from.
///
/// ## Corpus investigation (2026-03-20)
///
/// FN=1 from `Module.new do ... end` anonymous module blocks not being counted.
/// RuboCop's `on_casgn` handler matches `(casgn nil? _ (any_block (send (const
/// {nil? cbase} :Module) :new) ...))` — constant assignments where the value is
/// `Module.new do ... end`. Added handling for all constant assignment forms
/// (`ConstantWriteNode`, `ConstantPathWriteNode`, `ConstantOrWriteNode`,
/// `ConstantPathOrWriteNode`, `MultiWriteNode`) mirroring the ClassLength
/// pattern.
///
/// ## Corpus verification (2026-03-25)
///
/// verify_cop_locations.py: ALL FP/FN VERIFIED FIXED.
/// FP 0 fixed / 0 remain, FN 15 fixed / 0 remain.
pub struct ModuleLength;

fn is_top_level_module_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(read) = node.as_constant_read_node() {
        return read.name().as_slice() == b"Module";
    }
    if let Some(path) = node.as_constant_path_node() {
        return path.parent().is_none()
            && path
                .name()
                .map(|n| n.as_slice() == b"Module")
                .unwrap_or(false);
    }
    false
}

fn is_module_constructor(call: &ruby_prism::CallNode<'_>) -> bool {
    if call.name().as_slice() != b"new" {
        return false;
    }
    match call.receiver() {
        Some(r) => is_top_level_module_const(&r),
        None => false,
    }
}

fn assignment_module_constructor_call<'pr>(
    node: &ruby_prism::Node<'pr>,
) -> Option<ruby_prism::CallNode<'pr>> {
    let value = if let Some(n) = node.as_constant_write_node() {
        n.value()
    } else if let Some(n) = node.as_constant_path_write_node() {
        n.value()
    } else if let Some(n) = node.as_constant_or_write_node() {
        n.value()
    } else if let Some(n) = node.as_constant_path_or_write_node() {
        n.value()
    } else if let Some(n) = node.as_multi_write_node() {
        let has_constant_target = n.lefts().iter().any(|t| {
            t.as_constant_target_node().is_some() || t.as_constant_path_target_node().is_some()
        });
        if !has_constant_target {
            return None;
        }
        n.value()
    } else {
        return None;
    };

    let call = value.as_call_node()?;
    if !is_module_constructor(&call) {
        return None;
    }

    let has_block = call.block().and_then(|b| b.as_block_node()).is_some();
    if !has_block {
        return None;
    }

    Some(call)
}

/// Check if a module's body is exactly one class or module node (namespace module).
/// RuboCop skips namespace modules entirely (reports 0 length).
fn is_namespace_module(module_node: &ruby_prism::ModuleNode<'_>) -> bool {
    let body = match module_node.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => {
            // Body could also be a bare class/module node
            return body.as_class_node().is_some() || body.as_module_node().is_some();
        }
    };
    let body_nodes: Vec<_> = stmts.body().iter().collect();
    body_nodes.len() == 1
        && (body_nodes[0].as_class_node().is_some() || body_nodes[0].as_module_node().is_some())
}

impl Cop for ModuleLength {
    fn name(&self) -> &'static str {
        "Metrics/ModuleLength"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CLASS_NODE,
            CONSTANT_OR_WRITE_NODE,
            CONSTANT_PATH_OR_WRITE_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            MODULE_NODE,
            MULTI_WRITE_NODE,
            STATEMENTS_NODE,
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
        let max = config.get_usize("Max", 100);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        // Handle regular `module Foo ... end`
        if let Some(module_node) = node.as_module_node() {
            if is_namespace_module(&module_node) {
                return;
            }

            let start_offset = module_node.module_keyword_loc().start_offset();
            let end_offset = module_node.end_keyword_loc().start_offset();

            let mut foldable_ranges = Vec::new();
            if let Some(cao) = &count_as_one {
                if !cao.is_empty() {
                    if let Some(body) = module_node.body() {
                        foldable_ranges.extend(collect_foldable_ranges(source, &body, cao));
                    }
                }
            }

            let mut inner_ranges = Vec::new();
            if let Some(body) = module_node.body() {
                inner_ranges = inner_classlike_ranges(source, &body);
            }

            let count = count_body_lines_full(
                source,
                start_offset,
                end_offset,
                count_comments,
                &foldable_ranges,
                &inner_ranges,
            );

            if count > max {
                let (line, column) = source.offset_to_line_col(start_offset);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Module has too many lines. [{count}/{max}]"),
                ));
            }
            return;
        }

        // Handle `Foo = Module.new do ... end` (and variants)
        let Some(call_node) = assignment_module_constructor_call(node) else {
            return;
        };
        let Some(block_node) = call_node.block().and_then(|b| b.as_block_node()) else {
            return;
        };

        let start_offset = call_node.location().start_offset();
        let end_offset = block_node.closing_loc().start_offset();

        let mut foldable_ranges = Vec::new();
        if let Some(cao) = &count_as_one {
            if !cao.is_empty() {
                if let Some(body) = block_node.body() {
                    foldable_ranges.extend(collect_foldable_ranges(source, &body, cao));
                }
            }
        }

        let count = count_body_lines_ex(
            source,
            start_offset,
            end_offset,
            count_comments,
            &foldable_ranges,
        );

        if count > max {
            let (line, column) = source.offset_to_line_col(node.location().start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Module has too many lines. [{count}/{max}]"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ModuleLength, "cops/metrics/module_length");

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 4 body lines exceeds Max:3
        let source = b"module Foo\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n";
        let diags = run_cop_full_with_config(&ModuleLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:3 on 4-line module");
        assert!(diags[0].message.contains("[4/3]"));
    }

    #[test]
    fn singleton_class_lines_counted() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // Module with class << self containing 3 methods = 5 body lines
        // (class << self + 3 methods + end) but class << self should NOT be excluded
        let source = b"module Foo\n  class << self\n    def a; end\n    def b; end\n    def c; end\n  end\nend\n";
        let diags = run_cop_full_with_config(&ModuleLength, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire: class << self lines count toward module length"
        );
        assert!(
            diags[0].message.contains("[5/3]"),
            "Expected [5/3], got: {}",
            diags[0].message
        );
    }

    #[test]
    fn module_with_103_body_lines() {
        use crate::testutil::run_cop_full;

        // Synthetic module with 103 body lines (reproduces spork-like pattern)
        let mut src = String::from("module Spork\n");
        for i in 1..=103 {
            src.push_str(&format!("  x_{} = {}\n", i, i));
        }
        src.push_str("end\n");

        let diags = run_cop_full(&ModuleLength, src.as_bytes());
        assert!(
            !diags.is_empty(),
            "Should fire on module Spork (103 body lines > 100 max)"
        );
        assert!(
            diags[0].message.contains("[103/100]"),
            "Expected [103/100], got: {}",
            diags[0].message
        );
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
        let source = b"module Foo\n  a = 1\n  b = 2\n  ARR = [\n    1,\n    2\n  ]\nend\n";
        let diags = run_cop_full_with_config(&ModuleLength, source, config);
        assert!(
            diags.is_empty(),
            "Should not fire when array is folded (3/3)"
        );
    }
}
