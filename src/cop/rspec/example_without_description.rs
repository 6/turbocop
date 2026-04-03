use crate::cop::shared::node_type::{CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE};
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ExampleWithoutDescription;

/// Example methods that should have descriptions.
const EXAMPLE_METHODS: &[&[u8]] = &[b"it", b"specify", b"example"];

impl Cop for ExampleWithoutDescription {
    fn name(&self) -> &'static str {
        "RSpec/ExampleWithoutDescription"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if !EXAMPLE_METHODS.contains(&method_name) {
            return;
        }

        // Must have a block
        if call.block().is_none() {
            return;
        }

        let style = config.get_str("EnforcedStyle", "always_allow");
        if let Some(arguments) = call.arguments() {
            let arg_list: Vec<_> = arguments.arguments().iter().collect();

            // RuboCop's matcher flags only a *single* empty-string argument:
            //   it '' do ... end
            // It does not flag forms with additional metadata args, e.g.:
            //   it '', :aggregate_failures do ... end
            if arg_list.len() == 1 {
                if let Some(s) = arg_list[0].as_string_node() {
                    if s.unescaped().is_empty() {
                        let loc = s.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Omit the argument when you want to have auto-generated description."
                                .to_string(),
                        ));
                    }
                }
            }

            // Any arguments (description and/or metadata) mean this is not the
            // "missing description" form checked below.
            return;
        }

        // No description argument — behavior depends on EnforcedStyle
        match style {
            "always_allow" => {
                // No description is always OK
            }
            "disallow" => {
                // All examples must have descriptions,
                // but `specify` is always allowed when multiline
                if method_name == b"specify" {
                    let block = call.block().unwrap();
                    let block_loc = block.location();
                    let (start_line, _) = source.offset_to_line_col(block_loc.start_offset());
                    let end_off = block_loc
                        .end_offset()
                        .saturating_sub(1)
                        .max(block_loc.start_offset());
                    let (end_line, _) = source.offset_to_line_col(end_off);
                    if start_line != end_line {
                        return;
                    }
                }
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Add a description.".to_string(),
                ));
            }
            _ => {
                // "single_line_only": single-line OK, multi-line flagged
                let block = call.block().unwrap();
                let block_loc = block.location();
                let (start_line, _) = source.offset_to_line_col(block_loc.start_offset());
                let end_off = block_loc
                    .end_offset()
                    .saturating_sub(1)
                    .max(block_loc.start_offset());
                let (end_line, _) = source.offset_to_line_col(end_off);

                if start_line != end_line {
                    // RuboCop always allows `specify` without description when
                    // multiline, regardless of EnforcedStyle. See:
                    //   return if node.method?(:specify) && node.parent.multiline?
                    if method_name == b"specify" {
                        return;
                    }
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Add a description.".to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ExampleWithoutDescription,
        "cops/rspec/example_without_description"
    );
}
