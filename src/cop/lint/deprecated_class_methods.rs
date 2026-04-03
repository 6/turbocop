/// Lint/DeprecatedClassMethods
///
/// Detects deprecated class method usage and suggests replacements:
/// - `File.exists?` / `Dir.exists?` → `exist?`
/// - `ENV.clone` / `ENV.dup` → `ENV.to_h`
/// - `ENV.freeze` → `ENV`
/// - `iterator?` → `block_given?`
/// - `attr :name, true` → `attr_accessor :name`
/// - `attr :name, false` → `attr_reader :name`
/// - `Socket.gethostbyaddr` → `Addrinfo#getnameinfo`
/// - `Socket.gethostbyname` → `Addrinfo.getaddrinfo`
///
/// Investigation notes (2026-03):
/// Original implementation only handled File.exists? and Dir.exists?.
/// Added all remaining patterns from RuboCop's RESTRICT_ON_SEND list.
/// FN=60 was caused by missing ENV, iterator?, attr, and Socket patterns.
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=5, FN=0.
///
/// FP fixes:
/// - `constant_short_name()` matched only the final segment, so `Custom::File.exists?`
///   was incorrectly treated as `File.exists?`. Restrict bare constant receivers
///   to `File`, `Dir`, `ENV`, and `Socket`, with optional leading `::`.
/// - `ENV.clone` / `ENV.dup` / `ENV.freeze` were flagged even when arguments
///   were present. RuboCop only matches the zero-argument forms.
///
/// FN=0: no missing detections were reported in the corpus run.
// Handles both as_constant_read_node and as_constant_path_node (qualified constants like ::File)
use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DeprecatedClassMethods;

fn bare_or_cbase_constant(node: &ruby_prism::Node<'_>, expected: &[u8]) -> bool {
    if let Some(read) = node.as_constant_read_node() {
        return read.name().as_slice() == expected;
    }

    if let Some(path) = node.as_constant_path_node() {
        return path.parent().is_none()
            && path.name().is_some_and(|name| name.as_slice() == expected);
    }

    false
}

fn argument_count(call: &ruby_prism::CallNode<'_>) -> usize {
    call.arguments().map_or(0, |args| args.arguments().len())
}

impl Cop for DeprecatedClassMethods {
    fn name(&self) -> &'static str {
        "Lint/DeprecatedClassMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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

        let method_name = call.name().as_slice();

        // Handle receiver-less calls: `iterator?` and `attr :name, true/false`
        if call.receiver().is_none() {
            if method_name == b"iterator?" {
                // `iterator?` → `block_given?`
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let message = "`iterator?` is deprecated in favor of `block_given?`.".to_string();
                diagnostics.push(self.diagnostic(source, line, column, message));
                return;
            }

            if method_name == b"attr" {
                // `attr :name, true` → `attr_accessor :name`
                // `attr :name, false` → `attr_reader :name`
                // Only flag when the second argument is a boolean literal
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 2 {
                        let second = &arg_list[1];
                        let is_true = second.as_true_node().is_some();
                        let is_false = second.as_false_node().is_some();
                        if is_true || is_false {
                            let loc = call.location();
                            let call_source =
                                source.byte_slice(loc.start_offset(), loc.end_offset(), "attr");

                            let first_arg_source = {
                                let first = &arg_list[0];
                                let fl = first.location();
                                source.byte_slice(fl.start_offset(), fl.end_offset(), ":name")
                            };

                            let preferred = if is_true {
                                format!("attr_accessor {}", first_arg_source)
                            } else {
                                format!("attr_reader {}", first_arg_source)
                            };

                            let message = format!(
                                "`{}` is deprecated in favor of `{}`.",
                                call_source, preferred
                            );
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(source, line, column, message));
                        }
                    }
                }
                return;
            }

            return;
        }

        let receiver = call.receiver().unwrap();
        let receiver_loc = receiver.location();
        let receiver_source =
            source.byte_slice(receiver_loc.start_offset(), receiver_loc.end_offset(), "");

        match method_name {
            // File.exists? / Dir.exists?
            b"exists?"
                if argument_count(&call) == 1
                    && (bare_or_cbase_constant(&receiver, b"File")
                        || bare_or_cbase_constant(&receiver, b"Dir")) =>
            {
                let current = format!("{}.exists?", receiver_source);
                let prefer = format!("{}.exist?", receiver_source);
                let message = format!("`{}` is deprecated in favor of `{}`.", current, prefer);

                // Offense range: from receiver start to end of method selector
                let (line, column) = source.offset_to_line_col(receiver_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, message));
            }

            // ENV.clone / ENV.dup
            b"clone" | b"dup"
                if argument_count(&call) == 0 && bare_or_cbase_constant(&receiver, b"ENV") =>
            {
                let method_str = if method_name == b"clone" {
                    "clone"
                } else {
                    "dup"
                };
                let current = format!("{}.{}", receiver_source, method_str);
                let prefer = format!("{}.to_h", receiver_source);
                let message = format!("`{}` is deprecated in favor of `{}`.", current, prefer);

                let (line, column) = source.offset_to_line_col(receiver_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, message));
            }

            // ENV.freeze
            b"freeze"
                if argument_count(&call) == 0 && bare_or_cbase_constant(&receiver, b"ENV") =>
            {
                let current = format!("{}.freeze", receiver_source);
                let prefer = "ENV";
                let message = format!("`{}` is deprecated in favor of `{}`.", current, prefer);

                let (line, column) = source.offset_to_line_col(receiver_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, message));
            }

            // Socket.gethostbyaddr / Socket.gethostbyname
            b"gethostbyaddr" if bare_or_cbase_constant(&receiver, b"Socket") => {
                let current = format!("{}.gethostbyaddr", receiver_source);
                let message = format!(
                    "`{}` is deprecated in favor of `Addrinfo#getnameinfo`.",
                    current
                );

                let (line, column) = source.offset_to_line_col(receiver_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, message));
            }

            b"gethostbyname" if bare_or_cbase_constant(&receiver, b"Socket") => {
                let current = format!("{}.gethostbyname", receiver_source);
                let message = format!(
                    "`{}` is deprecated in favor of `Addrinfo.getaddrinfo`.",
                    current
                );

                let (line, column) = source.offset_to_line_col(receiver_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, message));
            }

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DeprecatedClassMethods, "cops/lint/deprecated_class_methods");
}
