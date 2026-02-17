use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct LinkToBlank;

const LINK_METHODS: &[&[u8]] = &[b"link_to", b"link_to_if", b"link_to_unless"];

fn has_target_blank(hash_elements: &ruby_prism::NodeList<'_>) -> bool {
    for elem in hash_elements.iter() {
        let assoc = match elem.as_assoc_node() {
            Some(a) => a,
            None => continue,
        };
        let key = assoc.key();
        let is_target_key = if let Some(sym) = key.as_symbol_node() {
            sym.unescaped() == b"target"
        } else if let Some(s) = key.as_string_node() {
            s.unescaped() == b"target"
        } else {
            false
        };
        if !is_target_key {
            continue;
        }
        let val = assoc.value();
        let is_blank = if let Some(s) = val.as_string_node() {
            s.unescaped() == b"_blank"
        } else if let Some(sym) = val.as_symbol_node() {
            sym.unescaped() == b"_blank"
        } else {
            false
        };
        if is_blank {
            return true;
        }
    }
    false
}

fn has_rel_noopener(hash_elements: &ruby_prism::NodeList<'_>) -> bool {
    for elem in hash_elements.iter() {
        let assoc = match elem.as_assoc_node() {
            Some(a) => a,
            None => continue,
        };
        let key = assoc.key();
        let is_rel_key = if let Some(sym) = key.as_symbol_node() {
            sym.unescaped() == b"rel"
        } else if let Some(s) = key.as_string_node() {
            s.unescaped() == b"rel"
        } else {
            false
        };
        if !is_rel_key {
            continue;
        }
        let val = assoc.value();
        let val_text = if let Some(s) = val.as_string_node() {
            Some(s.unescaped().to_vec())
        } else if let Some(sym) = val.as_symbol_node() {
            Some(sym.unescaped().to_vec())
        } else {
            None
        };
        if let Some(text) = val_text {
            let text_str = String::from_utf8_lossy(&text);
            let parts: Vec<&str> = text_str.split_whitespace().collect();
            if parts.contains(&"noopener") || parts.contains(&"noreferrer") {
                return true;
            }
        }
    }
    false
}

impl Cop for LinkToBlank {
    fn name(&self) -> &'static str {
        "Rails/LinkToBlank"
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
        if !LINK_METHODS.iter().any(|m| *m == name) {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Look for hash arguments containing target: '_blank'
        for arg in args.arguments().iter() {
            let elements = if let Some(hash) = arg.as_hash_node() {
                hash.elements()
            } else if let Some(kw) = arg.as_keyword_hash_node() {
                kw.elements()
            } else {
                continue;
            };

            if !has_target_blank(&elements) {
                continue;
            }

            if has_rel_noopener(&elements) {
                continue;
            }

            // Report on the entire hash arg that has target: _blank
            let loc = arg.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Specify a `:rel` option containing noopener.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LinkToBlank, "cops/rails/link_to_blank");
}
