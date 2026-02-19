use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashSet;
use crate::cop::node_type::{CALL_NODE, CLASS_NODE, MODULE_NODE, STATEMENTS_NODE, SYMBOL_NODE};

pub struct BisectedAttrAccessor;

impl Cop for BisectedAttrAccessor {
    fn name(&self) -> &'static str {
        "Style/BisectedAttrAccessor"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CLASS_NODE, MODULE_NODE, STATEMENTS_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let body = if let Some(class_node) = node.as_class_node() {
            class_node.body()
        } else if let Some(module_node) = node.as_module_node() {
            module_node.body()
        } else {
            return Vec::new();
        };

        let body = match body {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut readers: HashSet<String> = HashSet::new();
        let mut writers: HashSet<String> = HashSet::new();
        let mut reader_locs: Vec<(String, usize, usize)> = Vec::new();
        let mut writer_locs: Vec<(String, usize, usize)> = Vec::new();

        for stmt in stmts.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
                if call.receiver().is_some() {
                    continue;
                }

                let is_reader = name == "attr_reader" || name == "attr";
                let is_writer = name == "attr_writer";

                if !is_reader && !is_writer {
                    continue;
                }

                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        let attr_name = if let Some(sym) = arg.as_symbol_node() {
                            std::str::from_utf8(sym.unescaped())
                                .unwrap_or("")
                                .to_string()
                        } else {
                            continue;
                        };

                        let loc = arg.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());

                        if is_reader {
                            readers.insert(attr_name.clone());
                            reader_locs.push((attr_name, line, column));
                        } else {
                            writers.insert(attr_name.clone());
                            writer_locs.push((attr_name, line, column));
                        }
                    }
                }
            }
        }

        let mut diagnostics = Vec::new();
        let common: HashSet<_> = readers.intersection(&writers).collect();

        for attr_name in &common {
            for (name, line, column) in &reader_locs {
                if name == *attr_name {
                    diagnostics.push(self.diagnostic(
                        source,
                        *line,
                        *column,
                        format!("Combine both accessors into `attr_accessor :{}`.", attr_name),
                    ));
                }
            }
            for (name, line, column) in &writer_locs {
                if name == *attr_name {
                    diagnostics.push(self.diagnostic(
                        source,
                        *line,
                        *column,
                        format!("Combine both accessors into `attr_accessor :{}`.", attr_name),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BisectedAttrAccessor, "cops/style/bisected_attr_accessor");
}
