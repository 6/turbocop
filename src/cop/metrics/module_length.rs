use crate::cop::util::{count_body_lines_full, collect_foldable_ranges};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CLASS_NODE, MODULE_NODE, STATEMENTS_NODE};

pub struct ModuleLength;

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

/// Collect line ranges of inner class/module definitions within a body node.
/// Returns (start_line, end_line) pairs (1-indexed) for each inner class/module.
fn inner_classlike_ranges(source: &SourceFile, body: &ruby_prism::Node<'_>) -> Vec<(usize, usize)> {
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let mut ranges = Vec::new();
    for node in stmts.body().iter() {
        if let Some(cls) = node.as_class_node() {
            let loc = cls.location();
            let (start, _) = source.offset_to_line_col(loc.start_offset());
            let end_off = loc.end_offset().saturating_sub(1).max(loc.start_offset());
            let (end, _) = source.offset_to_line_col(end_off);
            ranges.push((start, end));
        } else if let Some(m) = node.as_module_node() {
            let loc = m.location();
            let (start, _) = source.offset_to_line_col(loc.start_offset());
            let end_off = loc.end_offset().saturating_sub(1).max(loc.start_offset());
            let (end, _) = source.offset_to_line_col(end_off);
            ranges.push((start, end));
        }
    }
    ranges
}

impl Cop for ModuleLength {
    fn name(&self) -> &'static str {
        "Metrics/ModuleLength"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, MODULE_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let module_node = match node.as_module_node() {
            Some(m) => m,
            None => return Vec::new(),
        };

        // Skip namespace modules (body is exactly one class or module)
        if is_namespace_module(&module_node) {
            return Vec::new();
        }

        let max = config.get_usize("Max", 100);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        let start_offset = module_node.module_keyword_loc().start_offset();
        let end_offset = module_node.end_keyword_loc().start_offset();

        // Collect foldable ranges from CountAsOne config
        let mut foldable_ranges = Vec::new();
        if let Some(cao) = &count_as_one {
            if !cao.is_empty() {
                if let Some(body) = module_node.body() {
                    foldable_ranges.extend(collect_foldable_ranges(source, &body, cao));
                }
            }
        }

        // Collect inner class/module line ranges to fully exclude from the count
        let mut inner_ranges = Vec::new();
        if let Some(body) = module_node.body() {
            inner_ranges = inner_classlike_ranges(source, &body);
        }

        let count = count_body_lines_full(source, start_offset, end_offset, count_comments, &foldable_ranges, &inner_ranges);

        if count > max {
            let (line, column) = source.offset_to_line_col(start_offset);
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Module has too many lines. [{count}/{max}]"),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ModuleLength, "cops/metrics/module_length");

    #[test]
    fn config_custom_max() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

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
    fn config_count_as_one_array() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                ("CountAsOne".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("array".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // Body: a, b, [\n1,\n2\n] = 2 + 1 folded = 3 lines
        let source = b"module Foo\n  a = 1\n  b = 2\n  ARR = [\n    1,\n    2\n  ]\nend\n";
        let diags = run_cop_full_with_config(&ModuleLength, source, config);
        assert!(diags.is_empty(), "Should not fire when array is folded (3/3)");
    }
}
