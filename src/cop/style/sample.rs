use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct Sample;

impl Cop for Sample {
    fn name(&self) -> &'static str {
        "Style/Sample"
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
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Must be .first or .last (the simple cases)
        if !matches!(method_bytes, b"first" | b"last") {
            return;
        }

        // Receiver must be a call to .shuffle
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if let Some(shuffle_call) = receiver.as_call_node() {
            if shuffle_call.name().as_slice() == b"shuffle" {
                // shuffle must have a receiver (the collection)
                if shuffle_call.receiver().is_none() {
                    return;
                }

                let loc = node.location();
                let incorrect = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                let (line, column) = source.offset_to_line_col(loc.start_offset());

                // Determine the correct replacement
                let correct = if call.arguments().is_some() {
                    let arg_src = call.arguments().map(|a| {
                        let args: Vec<_> = a.arguments().iter().collect();
                        if !args.is_empty() {
                            std::str::from_utf8(args[0].location().as_slice()).unwrap_or("").to_string()
                        } else {
                            String::new()
                        }
                    }).unwrap_or_default();

                    if shuffle_call.arguments().is_some() {
                        // shuffle has args (random:), include them
                        let shuffle_args = shuffle_call.arguments().map(|a| {
                            std::str::from_utf8(a.location().as_slice()).unwrap_or("")
                        }).unwrap_or("");
                        format!("sample({}, {})", arg_src, shuffle_args)
                    } else {
                        format!("sample({})", arg_src)
                    }
                } else if shuffle_call.arguments().is_some() {
                    let shuffle_args = shuffle_call.arguments().map(|a| {
                        std::str::from_utf8(a.location().as_slice()).unwrap_or("")
                    }).unwrap_or("");
                    format!("sample({})", shuffle_args)
                } else {
                    "sample".to_string()
                };

                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{}` instead of `{}`.", correct, incorrect),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Sample, "cops/style/sample");
}
