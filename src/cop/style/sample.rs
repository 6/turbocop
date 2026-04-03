use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects `shuffle.first`, `shuffle.last`, `shuffle[0]`, `shuffle[-1]`,
/// `shuffle[0, n]`, `shuffle[0..n]`, `shuffle[0...n]`, `shuffle.at(0)`,
/// `shuffle.at(-1)`, `shuffle.slice(0)`, `shuffle.slice(-1)`,
/// `shuffle.slice(0, n)`, `shuffle.slice(0..n)`, and `shuffle.slice(0...n)`.
///
/// ## Investigation (2026-03-29)
/// The first implementation only matched `first`/`last` and one-argument
/// `[]`/`at`/`slice` cases with `0` or `-1`, so corpus examples like
/// `shuffle[0, 4]`, `shuffle[0..2000]`, and `shuffle[0...1024]` were missed.
/// It also reported at `node.location()`, which starts at the full receiver
/// chain instead of the `shuffle` selector. On multiline receivers ending with
/// `.shuffle.first`, that produced a location mismatch counted as both FP and
/// FN. Fix: mirror RuboCop's sample-size rules for `[]`/`slice`, then report
/// from `shuffle_call.message_loc()` through the outer call end.
pub struct Sample;

#[derive(Clone, Copy)]
enum ShufflePattern {
    FirstLast,
    IndexAccess,
    At,
    Slice,
}

fn parse_integer_literal(node: &ruby_prism::Node<'_>) -> Option<i64> {
    let int = node.as_integer_node()?;
    let src = std::str::from_utf8(int.location().as_slice()).ok()?;
    src.replace('_', "").parse().ok()
}

fn range_sample_size(range: ruby_prism::RangeNode<'_>) -> Option<i64> {
    let low = match range.left() {
        Some(left) => parse_integer_literal(&left)?,
        None => 0,
    };
    let high = parse_integer_literal(&range.right()?)?;

    if low != 0 || high < 0 {
        return None;
    }

    Some(if range.is_exclude_end() {
        high - low
    } else {
        high - low + 1
    })
}

fn first_argument_source(
    source: &SourceFile,
    args: ruby_prism::ArgumentsNode<'_>,
) -> Option<String> {
    let arg = args.arguments().iter().next()?;
    Some(
        source
            .byte_slice(
                arg.location().start_offset(),
                arg.location().end_offset(),
                "",
            )
            .to_string(),
    )
}

fn index_sample_arg(
    call: &ruby_prism::CallNode<'_>,
    pattern: ShufflePattern,
) -> Option<Option<String>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();

    match pattern {
        ShufflePattern::At => {
            if arg_list.len() != 1 {
                return None;
            }

            match parse_integer_literal(&arg_list[0]) {
                Some(0) | Some(-1) => Some(None),
                _ => None,
            }
        }
        ShufflePattern::IndexAccess | ShufflePattern::Slice => match arg_list.len() {
            1 => {
                if let Some(range) = arg_list[0].as_range_node() {
                    return range_sample_size(range).map(|size| Some(size.to_string()));
                }

                match parse_integer_literal(&arg_list[0]) {
                    Some(0) | Some(-1) => Some(None),
                    _ => None,
                }
            }
            2 => {
                if parse_integer_literal(&arg_list[0])? != 0 {
                    return None;
                }

                let size = parse_integer_literal(&arg_list[1])?;
                Some(Some(size.to_string()))
            }
            _ => None,
        },
        ShufflePattern::FirstLast => None,
    }
}

fn format_replacement(sample_arg: Option<String>, shuffle_arg: Option<String>) -> String {
    let mut args = Vec::new();

    if let Some(sample_arg) = sample_arg.filter(|arg| !arg.is_empty()) {
        args.push(sample_arg);
    }

    if let Some(shuffle_arg) = shuffle_arg.filter(|arg| !arg.is_empty()) {
        args.push(shuffle_arg);
    }

    if args.is_empty() {
        "sample".to_string()
    } else {
        format!("sample({})", args.join(", "))
    }
}

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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(call) => call,
            None => return,
        };

        let pattern = match call.name().as_slice() {
            b"first" | b"last" => ShufflePattern::FirstLast,
            b"[]" => ShufflePattern::IndexAccess,
            b"at" => ShufflePattern::At,
            b"slice" => ShufflePattern::Slice,
            _ => return,
        };

        let receiver = match call.receiver() {
            Some(receiver) => receiver,
            None => return,
        };

        let shuffle_call = match receiver.as_call_node() {
            Some(call) if call.name().as_slice() == b"shuffle" && call.receiver().is_some() => call,
            _ => return,
        };

        let sample_arg = match pattern {
            ShufflePattern::FirstLast => call
                .arguments()
                .and_then(|args| first_argument_source(source, args)),
            ShufflePattern::IndexAccess | ShufflePattern::At | ShufflePattern::Slice => {
                match index_sample_arg(&call, pattern) {
                    Some(sample_arg) => sample_arg,
                    None => return,
                }
            }
        };

        let shuffle_arg = shuffle_call
            .arguments()
            .map(|args| {
                source.byte_slice(
                    args.location().start_offset(),
                    args.location().end_offset(),
                    "",
                )
            })
            .map(str::to_string);

        let correct = format_replacement(sample_arg, shuffle_arg);
        let offense_start = shuffle_call
            .message_loc()
            .unwrap_or_else(|| shuffle_call.location())
            .start_offset();
        let offense_end = call.location().end_offset();
        let incorrect = source.byte_slice(offense_start, offense_end, "");
        let (line, column) = source.offset_to_line_col(offense_start);

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{correct}` instead of `{incorrect}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Sample, "cops/style/sample");
}
