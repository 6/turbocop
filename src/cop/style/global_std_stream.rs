use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct GlobalStdStream;

impl Cop for GlobalStdStream {
    fn name(&self) -> &'static str {
        "Style/GlobalStdStream"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = GlobalStdStreamVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_gvar_assignment: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct GlobalStdStreamVisitor<'a, 'src> {
    cop: &'a GlobalStdStream,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when visiting the value side of a `$stdout = ...` assignment
    in_gvar_assignment: bool,
}

impl GlobalStdStreamVisitor<'_, '_> {
    fn check_std_stream(&mut self, name_bytes: &[u8], loc: &ruby_prism::Location<'_>) {
        if self.in_gvar_assignment {
            return;
        }
        if let Some(gvar) = std_stream_gvar(name_bytes) {
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let const_name = std::str::from_utf8(name_bytes).unwrap_or("");
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Use `{}` instead of `{}`.", gvar, const_name),
            ));
        }
    }
}

impl Visit<'_> for GlobalStdStreamVisitor<'_, '_> {
    fn visit_global_variable_write_node(&mut self, node: &ruby_prism::GlobalVariableWriteNode<'_>) {
        let var_name = node.name();
        let var_bytes = var_name.as_slice();
        // Check if this is $stdout = ..., $stderr = ..., or $stdin = ...
        let is_std_gvar = matches!(var_bytes, b"$stdout" | b"$stderr" | b"$stdin");
        if is_std_gvar {
            self.in_gvar_assignment = true;
        }
        ruby_prism::visit_global_variable_write_node(self, node);
        if is_std_gvar {
            self.in_gvar_assignment = false;
        }
    }

    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'_>) {
        let name = node.name();
        self.check_std_stream(name.as_slice(), &node.location());
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'_>) {
        // Must be top-level (::STDOUT) — parent is None
        if node.parent().is_some() {
            ruby_prism::visit_constant_path_node(self, node);
            return;
        }
        if let Some(name) = node.name() {
            self.check_std_stream(name.as_slice(), &node.location());
        }
        // Don't visit children — we already handled it
    }
}

fn std_stream_gvar(name: &[u8]) -> Option<&'static str> {
    match name {
        b"STDOUT" => Some("$stdout"),
        b"STDERR" => Some("$stderr"),
        b"STDIN" => Some("$stdin"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(GlobalStdStream, "cops/style/global_std_stream");
}
