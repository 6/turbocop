use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct WhereEquals;

impl Cop for WhereEquals {
    fn name(&self) -> &'static str {
        "Rails/WhereEquals"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();
        if name != b"where" && name != b"not" {
            return Vec::new();
        }

        // If `not`, check that receiver is a `where` call
        if name == b"not" {
            if let Some(recv) = call.receiver() {
                if let Some(recv_call) = recv.as_call_node() {
                    if recv_call.name().as_slice() != b"where" {
                        return Vec::new();
                    }
                } else {
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // First argument must be a string literal with a simple comparison pattern
        let str_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let template = std::str::from_utf8(str_node.unescaped()).unwrap_or("");

        // Check patterns:
        // column = ?
        // column IS NULL
        // column IN (?)
        let eq_anon = regex::Regex::new(r"^[\w.]+\s+=\s+\?$").unwrap();
        let in_anon = regex::Regex::new(r"(?i)^[\w.]+\s+IN\s+\(\?\)$").unwrap();
        let is_null = regex::Regex::new(r"(?i)^[\w.]+\s+IS\s+NULL$").unwrap();
        let eq_named = regex::Regex::new(r"^[\w.]+\s+=\s+:\w+$").unwrap();
        let in_named = regex::Regex::new(r"(?i)^[\w.]+\s+IN\s+\(:\w+\)$").unwrap();

        let is_simple_sql = eq_anon.is_match(template)
            || in_anon.is_match(template)
            || is_null.is_match(template)
            || eq_named.is_match(template)
            || in_named.is_match(template);

        if !is_simple_sql {
            return Vec::new();
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let method = std::str::from_utf8(name).unwrap_or("where");
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `{method}(attribute: value)` instead of manually constructing SQL."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereEquals, "cops/rails/where_equals");
}
