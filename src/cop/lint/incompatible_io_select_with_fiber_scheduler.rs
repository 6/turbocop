use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, NIL_NODE};
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for `IO.select` that is incompatible with Fiber Scheduler.
/// Suggests using `io.wait_readable` or `io.wait_writable` instead.
pub struct IncompatibleIoSelectWithFiberScheduler;

impl Cop for IncompatibleIoSelectWithFiberScheduler {
    fn name(&self) -> &'static str {
        "Lint/IncompatibleIoSelectWithFiberScheduler"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, NIL_NODE]
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

        if call.name().as_slice() != b"select" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_name = match constant_name(&receiver) {
            Some(n) => n,
            None => return,
        };

        if recv_name != b"IO" {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args: Vec<_> = arguments.arguments().iter().collect();
        if args.is_empty() || args.len() > 4 {
            return;
        }

        // args: [read_array, write_array, except_array, timeout]
        let read = args.first();
        let write = args.get(1);
        let excepts = args.get(2);

        // If excepts has elements, skip (no alternative API)
        if let Some(exc) = excepts {
            if let Some(arr) = exc.as_array_node() {
                if !arr.elements().is_empty() {
                    return;
                }
            }
            // If excepts is not an empty array and not nil, skip
            else if exc.as_nil_node().is_none() {
                return;
            }
        }

        // Check if it's a readable or writable pattern
        let is_read = is_single_element_array(read) && is_empty_or_nil(write);
        let is_write = is_single_element_array(write) && is_empty_or_nil(read);

        if !is_read && !is_write {
            return;
        }

        let call_src = node_source(source, &node);
        let preferred = if is_read {
            let io_src = single_array_element_source(read.unwrap(), source);
            let timeout = args.get(3).map(|t| node_source(source, t));
            if let Some(t) = timeout {
                format!("{}.wait_readable({})", io_src, t)
            } else {
                format!("{}.wait_readable", io_src)
            }
        } else {
            let io_src = single_array_element_source(write.unwrap(), source);
            let timeout = args.get(3).map(|t| node_source(source, t));
            if let Some(t) = timeout {
                format!("{}.wait_writable({})", io_src, t)
            } else {
                format!("{}.wait_writable", io_src)
            }
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}` instead of `{}`.", preferred, call_src),
        ));
    }
}

fn is_single_element_array(node: Option<&ruby_prism::Node<'_>>) -> bool {
    match node {
        Some(n) => {
            if let Some(arr) = n.as_array_node() {
                arr.elements().len() == 1
            } else {
                false
            }
        }
        None => false,
    }
}

fn is_empty_or_nil(node: Option<&ruby_prism::Node<'_>>) -> bool {
    match node {
        None => true,
        Some(n) => {
            if n.as_nil_node().is_some() {
                return true;
            }
            if let Some(arr) = n.as_array_node() {
                return arr.elements().is_empty();
            }
            false
        }
    }
}

fn single_array_element_source<'a>(node: &ruby_prism::Node<'_>, source: &'a SourceFile) -> &'a str {
    if let Some(arr) = node.as_array_node() {
        let elements: Vec<_> = arr.elements().iter().collect();
        if elements.len() == 1 {
            return node_source(source, &elements[0]);
        }
    }
    "io"
}

fn node_source<'a>(source: &'a SourceFile, node: &ruby_prism::Node<'_>) -> &'a str {
    let loc = node.location();
    std::str::from_utf8(&source.as_bytes()[loc.start_offset()..loc.end_offset()]).unwrap_or("...")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IncompatibleIoSelectWithFiberScheduler,
        "cops/lint/incompatible_io_select_with_fiber_scheduler"
    );
}
