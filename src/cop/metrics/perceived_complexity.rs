use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct PerceivedComplexity;

#[derive(Default)]
struct PerceivedCounter {
    complexity: usize,
}

impl PerceivedCounter {
    fn count_node(&mut self, node: &ruby_prism::Node<'_>) {
        match node {
            ruby_prism::Node::IfNode { .. }
            | ruby_prism::Node::WhileNode { .. }
            | ruby_prism::Node::UntilNode { .. }
            | ruby_prism::Node::ForNode { .. }
            | ruby_prism::Node::WhenNode { .. }
            | ruby_prism::Node::RescueNode { .. }
            | ruby_prism::Node::ElseNode { .. } => {
                self.complexity += 1;
            }
            _ => {}
        }
    }
}

impl<'pr> Visit<'pr> for PerceivedCounter {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.count_node(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.count_node(&node);
    }
}

impl Cop for PerceivedComplexity {
    fn name(&self) -> &'static str {
        "Metrics/PerceivedComplexity"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let max = config
            .options
            .get("Max")
            .and_then(|v| v.as_u64())
            .unwrap_or(8) as usize;

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut counter = PerceivedCounter::default();
        counter.visit(&body);

        let score = 1 + counter.complexity;
        if score > max {
            let method_name =
                std::str::from_utf8(def_node.name().as_slice()).unwrap_or("unknown");
            let start_offset = def_node.def_keyword_loc().start_offset();
            let (line, column) = source.offset_to_line_col(start_offset);
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: format!(
                    "Perceived complexity for {method_name} is too high. [{score}/{max}]"
                ),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &PerceivedComplexity,
            include_bytes!("../../../testdata/cops/metrics/perceived_complexity/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &PerceivedComplexity,
            include_bytes!("../../../testdata/cops/metrics/perceived_complexity/no_offense.rb"),
        );
    }

    #[test]
    fn config_custom_max() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // 1 (base) + 1 (if) + 1 (else) = 3 > Max:1
        let source = b"def foo\n  if x\n    y\n  else\n    z\n  end\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:1 on method with if/else");
        assert!(diags[0].message.contains("/1]"));
    }
}
