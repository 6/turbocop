use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ImplicitSubject;

/// RSpec example method names (it, specify, example, scenario, its, etc.)
const EXAMPLE_METHODS: &[&[u8]] = &[
    b"it",
    b"specify",
    b"example",
    b"scenario",
    b"its",
    b"xit",
    b"xspecify",
    b"xexample",
    b"xscenario",
    b"fit",
    b"fspecify",
    b"fexample",
    b"fscenario",
    b"skip",
    b"pending",
];

/// Check if trimmed line starts with an RSpec example method call (e.g. `it `, `its(`, `specify {`).
/// Returns the example method name if found, or None.
fn line_example_method(trimmed: &[u8]) -> Option<&'static [u8]> {
    for &method in EXAMPLE_METHODS {
        if trimmed.len() > method.len() && trimmed.starts_with(method) {
            let next = trimmed[method.len()];
            // Method name must be followed by space, `(`, `{`, or `'`/`"`
            if next == b' ' || next == b'(' || next == b'{' || next == b'\'' || next == b'"' {
                return Some(method);
            }
        }
    }
    None
}

impl Cop for ImplicitSubject {
    fn name(&self) -> &'static str {
        "RSpec/ImplicitSubject"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        // Config: EnforcedStyle — "single_line_only" (default), "single_statement_only", or "disallow"
        let enforced_style = config.get_str("EnforcedStyle", "single_line_only");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();

        let is_implicit = method_name == b"is_expected"
            || method_name == b"should"
            || method_name == b"should_not";

        if !is_implicit {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        // Find the enclosing example block by scanning backward through source lines.
        // We look for the nearest line that starts with an RSpec example method.
        let enclosing = find_enclosing_example(source, line);

        // If inside an `its` block, implicit subject is always allowed
        // (RuboCop exempts `its` in all styles).
        if let Some((method, _)) = enclosing {
            if method == b"its" {
                return;
            }
        }

        // "disallow" style: flag all implicit subject usage (except `its` handled above)
        if enforced_style == "disallow" {
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Don't use implicit subject.".to_string(),
            ));
            return;
        }

        // Determine if this is a single-line example block
        let is_single_line = match enclosing {
            Some((_, example_line)) => {
                // Check if the example block is on a single line
                let example_line_bytes = source.lines().nth(example_line - 1).unwrap_or(b"");
                // Single-line if the example opener line contains a closing `}` or `end`
                // and the implicit subject call is on the same line as the example opener
                line == example_line && example_line_bytes.contains(&b'}')
            }
            None => false,
        };

        match enforced_style {
            "single_line_only" => {
                if is_single_line {
                    return; // Single-line example — allowed
                }
            }
            "single_statement_only" => {
                if is_single_line {
                    return; // Single-line is always single-statement
                }
                // Multi-line but single-statement: check if there's only one statement
                // by seeing if the implicit subject line is the only statement line in the block.
                if let Some((_, example_line)) = enclosing {
                    if is_single_statement_block(source, example_line, line) {
                        return; // Single-statement multi-line — allowed
                    }
                }
            }
            _ => {}
        }

        // This is used in a context that should be flagged
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Don't use implicit subject.".to_string(),
        ));
    }
}

/// Find the enclosing RSpec example block by scanning backward from the given line.
/// Returns the example method name and the line number of the example opener, or None.
fn find_enclosing_example(source: &SourceFile, from_line: usize) -> Option<(&'static [u8], usize)> {
    // First check the same line (single-line examples like `it { is_expected.to ... }`)
    for check_line in (1..=from_line).rev() {
        let line_bytes = source.lines().nth(check_line - 1).unwrap_or(b"");
        let trimmed = match line_bytes.iter().position(|&b| b != b' ' && b != b'\t') {
            Some(s) => &line_bytes[s..],
            None => continue,
        };

        if let Some(method) = line_example_method(trimmed) {
            return Some((method, check_line));
        }

        // Stop scanning if we hit a line that looks like a block-level construct
        // (class, module, describe, context, def) to avoid crossing block boundaries.
        if trimmed.starts_with(b"describe ")
            || trimmed.starts_with(b"context ")
            || trimmed.starts_with(b"RSpec.")
            || trimmed.starts_with(b"class ")
            || trimmed.starts_with(b"module ")
        {
            return None;
        }
    }
    None
}

/// Check if a multi-line example block contains only a single statement.
/// `example_line` is the line of the `it`/`specify`/etc. opener.
/// `call_line` is the line of the implicit subject call.
/// A single-statement block has exactly one non-blank, non-end line between the opener and closing.
fn is_single_statement_block(source: &SourceFile, example_line: usize, call_line: usize) -> bool {
    // Count non-blank statement lines between the example opener and the end
    let mut statement_count = 0;
    let total_lines = source.lines().count();

    for check_line in (example_line + 1)..=total_lines {
        let line_bytes = source.lines().nth(check_line - 1).unwrap_or(b"");
        let trimmed = match line_bytes.iter().position(|&b| b != b' ' && b != b'\t') {
            Some(s) => &line_bytes[s..],
            None => continue, // blank line
        };

        // Stop at the closing `end`
        if trimmed == b"end" || trimmed.starts_with(b"end ") || trimmed.starts_with(b"end\n") {
            break;
        }

        statement_count += 1;
    }

    // The call_line should be in a block with exactly one statement
    let _ = call_line; // used for context; we count all statements
    statement_count == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ImplicitSubject, "cops/rspec/implicit_subject");

    #[test]
    fn disallow_style_flags_single_line_too() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("disallow".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"it { is_expected.to eq(1) }\n";
        let diags = crate::testutil::run_cop_full_with_config(&ImplicitSubject, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Don't use implicit subject"));
    }

    #[test]
    fn disallow_style_allows_its_blocks() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("disallow".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"its(:quality) { is_expected.to be :high }\n";
        let diags = crate::testutil::run_cop_full_with_config(&ImplicitSubject, source, config);
        assert_eq!(
            diags.len(),
            0,
            "its blocks should be exempt even with disallow style"
        );
    }

    #[test]
    fn single_statement_only_allows_single_statement() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("single_statement_only".into()),
            )]),
            ..CopConfig::default()
        };
        // Single statement in multi-line block — should be allowed
        let source = b"it 'checks' do\n  is_expected.to be_good\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ImplicitSubject, source, config);
        assert_eq!(
            diags.len(),
            0,
            "single-statement multi-line should be allowed"
        );
    }

    #[test]
    fn single_statement_only_flags_multi_statement() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("single_statement_only".into()),
            )]),
            ..CopConfig::default()
        };
        // Multiple statements in multi-line block — should be flagged
        let source = b"it 'checks' do\n  subject.age = 18\n  is_expected.to be_valid\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ImplicitSubject, source, config);
        assert_eq!(diags.len(), 1, "multi-statement should be flagged");
    }
}
