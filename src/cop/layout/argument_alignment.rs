use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct ArgumentAlignment;

impl Cop for ArgumentAlignment {
    fn name(&self) -> &'static str {
        "Layout/ArgumentAlignment"
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
        let style = config.get_str("EnforcedStyle", "with_first_argument");
        let indent_width = config.get_usize("IndentationWidth", 2);
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // RuboCop skips []= calls (bracket assignment)
        if call_node.name().as_slice() == b"[]=" {
            return;
        }

        let arguments = match call_node.arguments() {
            Some(args) => args,
            None => return,
        };

        let arg_list = arguments.arguments();
        if arg_list.is_empty() {
            return;
        }

        // Collect effective arguments: if the only argument is a KeywordHashNode,
        // use its elements as individual alignment targets (RuboCop checks each
        // key-value pair in a keyword hash for alignment).
        let effective_args: Vec<ruby_prism::Node<'_>> = if arg_list.len() == 1 {
            let first = arg_list.iter().next().unwrap();
            if let Some(kw_hash) = first.as_keyword_hash_node() {
                kw_hash.elements().iter().collect()
            } else {
                arg_list.iter().collect()
            }
        } else {
            arg_list.iter().collect()
        };

        if effective_args.len() < 2 {
            return;
        }

        let first_arg = &effective_args[0];
        let (first_line, first_col) = source.offset_to_line_col(first_arg.location().start_offset());

        let mut checked_lines = std::collections::HashSet::new();
        checked_lines.insert(first_line);

        // For "with_fixed_indentation", the expected column is the call line's
        // indentation + indent_width
        let expected_col = match style {
            "with_fixed_indentation" => {
                // Use the line containing the method selector (or opening paren),
                // NOT the full call expression start (which includes the receiver
                // chain). For chained calls like `Foo.bar.baz("str", arg)`, the
                // call node starts at `Foo` but we want the indentation of the
                // line containing `.baz(`.
                let base_line = if let Some(open_loc) = call_node.opening_loc() {
                    source.offset_to_line_col(open_loc.start_offset()).0
                } else if let Some(msg_loc) = call_node.message_loc() {
                    source.offset_to_line_col(msg_loc.start_offset()).0
                } else {
                    source.offset_to_line_col(call_node.location().start_offset()).0
                };
                let base_line_bytes = source.lines().nth(base_line - 1).unwrap_or(b"");
                crate::cop::util::indentation_of(base_line_bytes) + indent_width
            }
            _ => first_col, // "with_first_argument" (default)
        };

        for arg in effective_args.iter().skip(1) {
            let (arg_line, arg_col) = source.offset_to_line_col(arg.location().start_offset());
            // Only check the FIRST argument on each new line
            if !checked_lines.contains(&arg_line) {
                checked_lines.insert(arg_line);
                // Skip arguments that don't begin their line (matching RuboCop's
                // begins_its_line? check). For example, in `}, suffix: :action`
                // the suffix: argument is not first on its line.
                if !crate::cop::util::begins_its_line(source, arg.location().start_offset()) {
                    continue;
                }
                if arg_col != expected_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        arg_line,
                        arg_col,
                        "Align the arguments of a method call if they span more than one line."
                            .to_string(),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(ArgumentAlignment, "cops/layout/argument_alignment");

    #[test]
    fn single_line_call_no_offense() {
        let source = b"foo(1, 2, 3)\n";
        let diags = run_cop_full(&ArgumentAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn with_fixed_indentation_style() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("with_fixed_indentation".into())),
            ]),
            ..CopConfig::default()
        };
        // Args aligned with first arg (column 4) but with_fixed_indentation expects column 2
        let src = b"foo(1,\n    2)\n";
        let diags = run_cop_full_with_config(&ArgumentAlignment, src, config.clone());
        assert_eq!(diags.len(), 1, "with_fixed_indentation should flag args aligned with first arg");

        // Args at fixed indentation (2 spaces from call)
        let src2 = b"foo(1,\n  2)\n";
        let diags2 = run_cop_full_with_config(&ArgumentAlignment, src2, config);
        assert!(diags2.is_empty(), "with_fixed_indentation should accept fixed-indent args");
    }
}
