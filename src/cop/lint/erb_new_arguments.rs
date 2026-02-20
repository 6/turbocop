use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};

/// Checks for deprecated `ERB.new` with positional arguments beyond the first.
/// Since Ruby 2.6, non-keyword arguments other than the first one are deprecated.
pub struct ErbNewArguments;

impl Cop for ErbNewArguments {
    fn name(&self) -> &'static str {
        "Lint/ErbNewArguments"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"new" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let name = match constant_name(&receiver) {
            Some(n) => n,
            None => return,
        };

        if name != b"ERB" {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args: Vec<_> = arguments.arguments().iter().collect();

        // ERB.new(str) or ERB.new(str, key: val) are fine
        if args.len() <= 1 {
            return;
        }
        if args.len() == 2 && args[1].as_keyword_hash_node().is_some() {
            return;
        }


        // Check args at positions 1, 2, 3 (safe_level, trim_mode, eoutvar)
        for (i, arg) in args.iter().enumerate().skip(1).take(3) {
            // Skip if it's a hash (keyword args)
            if arg.as_keyword_hash_node().is_some() || arg.as_hash_node().is_some() {
                continue;
            }

            let msg = match i {
                1 => "Passing safe_level with the 2nd argument of `ERB.new` is deprecated. Do not use it, and specify other arguments as keyword arguments.".to_string(),
                2 => {
                    let arg_src = std::str::from_utf8(
                        &source.as_bytes()[arg.location().start_offset()..arg.location().end_offset()],
                    ).unwrap_or("...");
                    format!("Passing trim_mode with the 3rd argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, trim_mode: {})` instead.", arg_src)
                }
                3 => {
                    let arg_src = std::str::from_utf8(
                        &source.as_bytes()[arg.location().start_offset()..arg.location().end_offset()],
                    ).unwrap_or("...");
                    format!("Passing eoutvar with the 4th argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, eoutvar: {})` instead.", arg_src)
                }
                _ => continue,
            };

            let loc = arg.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(source, line, column, msg));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ErbNewArguments, "cops/lint/erb_new_arguments");
}
