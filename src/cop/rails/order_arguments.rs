use crate::cop::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct OrderArguments;

impl Cop for OrderArguments {
    fn name(&self) -> &'static str {
        "Rails/OrderArguments"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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

        if call.name().as_slice() != b"order" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // All arguments must be string literals
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check if all arguments are string literals that can be converted to symbols
        let re = regex::Regex::new(r"(?i)^(\w+)\s*(asc|desc)?$").unwrap();
        let mut all_strings = true;
        let mut all_convertible = true;

        for arg in &arg_list {
            if let Some(str_node) = arg.as_string_node() {
                let value = str_node.unescaped();
                let text = std::str::from_utf8(value).unwrap_or("");
                // Check each comma-separated part
                for part in text.split(',') {
                    let trimmed = part.trim();
                    if !re.is_match(trimmed) {
                        all_convertible = false;
                        break;
                    }
                    // Check for positional columns (numeric)
                    let caps = re.captures(trimmed).unwrap();
                    let col = caps.get(1).unwrap().as_str();
                    if col.chars().all(|c| c.is_ascii_digit()) {
                        all_convertible = false;
                        break;
                    }
                }
            } else {
                all_strings = false;
            }
        }

        if !all_strings || !all_convertible {
            return;
        }

        // Build the preferred representation
        let mut preferred_parts = Vec::new();
        let mut use_hash = false;

        for arg in &arg_list {
            if let Some(str_node) = arg.as_string_node() {
                let value = str_node.unescaped();
                let text = std::str::from_utf8(value).unwrap_or("");
                for part in text.split(',') {
                    let trimmed = part.trim();
                    let caps = re.captures(trimmed).unwrap();
                    let col = caps.get(1).unwrap().as_str().to_lowercase();
                    let dir = caps.get(2).map(|m| m.as_str().to_lowercase());
                    let direction = dir.as_deref().unwrap_or("asc");
                    if direction == "asc" && !use_hash {
                        preferred_parts.push(format!(":{col}"));
                    } else {
                        use_hash = true;
                        preferred_parts.push(format!("{col}: :{direction}"));
                    }
                }
            }
        }

        let prefer = preferred_parts.join(", ");

        let first_arg_loc = arg_list[0].location();
        let (line, column) = source.offset_to_line_col(first_arg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{prefer}` instead."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OrderArguments, "cops/rails/order_arguments");
}
