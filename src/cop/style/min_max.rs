use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MinMax;

impl Cop for MinMax {
    fn name(&self) -> &'static str {
        "Style/MinMax"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let elements: Vec<_> = array_node.elements().iter().collect();
        if elements.len() != 2 {
            return Vec::new();
        }

        // First element must be receiver.min
        let min_recv_src = match get_receiver_of_method(&elements[0], b"min", source) {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Second element must be receiver.max (same receiver)
        let max_recv_src = match get_receiver_of_method(&elements[1], b"max", source) {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Receivers must be the same
        if min_recv_src != max_recv_src || min_recv_src.is_empty() {
            return Vec::new();
        }

        let loc = array_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let array_src = std::str::from_utf8(loc.as_slice()).unwrap_or("...");
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}.minmax` instead of `{}`.", min_recv_src, array_src),
        )]
    }
}

fn get_receiver_of_method<'a>(
    node: &ruby_prism::Node<'a>,
    method_name: &[u8],
    source: &'a SourceFile,
) -> Option<String> {
    let call = node.as_call_node()?;
    if call.name().as_slice() != method_name {
        return None;
    }
    if call.arguments().is_some() {
        return None;
    }
    let recv = call.receiver()?;
    let recv_loc = recv.location();
    let recv_src = std::str::from_utf8(&source.as_bytes()[recv_loc.start_offset()..recv_loc.end_offset()])
        .unwrap_or("")
        .to_string();
    Some(recv_src)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MinMax, "cops/style/min_max");
}
