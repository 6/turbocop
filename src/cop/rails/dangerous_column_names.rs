use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DangerousColumnNames;

const DANGEROUS_NAMES: &[&[u8]] = &[
    b"type",
    b"id",
    b"created_at",
    b"updated_at",
    b"attributes",
    b"class",
    b"new",
    b"save",
    b"destroy",
    b"delete",
    b"update",
    b"valid",
    b"errors",
    b"lock_version",
];

impl Cop for DangerousColumnNames {
    fn name(&self) -> &'static str {
        "Rails/DangerousColumnNames"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["db/migrate/**/*.rb"]
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

        if call.name().as_slice() != b"add_column" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return Vec::new();
        }

        // Second arg is the column name (symbol)
        let name_str = match arg_list[1].as_symbol_node() {
            Some(s) => {
                let col_name = s.unescaped();
                if !DANGEROUS_NAMES.contains(&col_name) {
                    return Vec::new();
                }
                String::from_utf8_lossy(col_name).to_string()
            }
            None => return Vec::new(),
        };
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Avoid using `{name_str}` as a column name. It conflicts with ActiveRecord internals."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DangerousColumnNames, "cops/rails/dangerous_column_names");
}
