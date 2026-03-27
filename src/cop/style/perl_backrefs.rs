use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/PerlBackrefs: flags Perl-style regexp backreferences and their English
/// aliases in favor of `Regexp.last_match`.
///
/// ## Investigation findings (2026-03)
///
/// ### FN root cause (84 total)
/// RuboCop also flags the English regexp globals `$MATCH`, `$PREMATCH`,
/// `$POSTMATCH`, and `$LAST_PAREN_MATCH`. Prism parses those as
/// `GlobalVariableReadNode`, but the original implementation only listened for
/// `BackReferenceReadNode` and `NumberedReferenceReadNode`, so those aliases
/// were missed entirely.
///
/// ### Namespace-sensitive replacement text
/// Inside class/module scopes, RuboCop suggests `::Regexp.last_match...` to
/// avoid constant shadowing. The original implementation always emitted
/// `Regexp.last_match...`, so namespaced fixture coverage now checks the
/// `::`-prefixed replacement text for all PerlBackrefs variants.
pub struct PerlBackrefs;

impl Cop for PerlBackrefs {
    fn name(&self) -> &'static str {
        "Style/PerlBackrefs"
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
        let mut visitor = PerlBackrefsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            namespace_depth: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct PerlBackrefsVisitor<'a> {
    cop: &'a PerlBackrefs,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    namespace_depth: usize,
}

impl PerlBackrefsVisitor<'_> {
    fn add_offense(&mut self, loc: ruby_prism::Location<'_>, replacement: &str, var_display: &str) {
        let prefix = if self.namespace_depth > 0 { "::" } else { "" };
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!("Prefer `{prefix}{replacement}` over `{var_display}`."),
        ));
    }
}

impl<'pr> Visit<'pr> for PerlBackrefsVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        self.namespace_depth += 1;
        ruby_prism::visit_class_node(self, node);
        self.namespace_depth -= 1;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.namespace_depth += 1;
        ruby_prism::visit_module_node(self, node);
        self.namespace_depth -= 1;
    }

    fn visit_back_reference_read_node(&mut self, node: &ruby_prism::BackReferenceReadNode<'pr>) {
        let (replacement, var_display) = match node.name().as_slice() {
            b"$&" => ("Regexp.last_match(0)", "$&"),
            b"$`" => ("Regexp.last_match.pre_match", "$`"),
            b"$'" => ("Regexp.last_match.post_match", "$'"),
            b"$+" => ("Regexp.last_match(-1)", "$+"),
            _ => return,
        };
        self.add_offense(node.location(), replacement, var_display);
    }

    fn visit_global_variable_read_node(&mut self, node: &ruby_prism::GlobalVariableReadNode<'pr>) {
        let (replacement, var_display) = match node.name().as_slice() {
            b"$MATCH" => ("Regexp.last_match(0)", "$MATCH"),
            b"$PREMATCH" => ("Regexp.last_match.pre_match", "$PREMATCH"),
            b"$POSTMATCH" => ("Regexp.last_match.post_match", "$POSTMATCH"),
            b"$LAST_PAREN_MATCH" => ("Regexp.last_match(-1)", "$LAST_PAREN_MATCH"),
            _ => return,
        };
        self.add_offense(node.location(), replacement, var_display);
    }

    fn visit_numbered_reference_read_node(
        &mut self,
        node: &ruby_prism::NumberedReferenceReadNode<'pr>,
    ) {
        let num = node.number();
        self.add_offense(
            node.location(),
            &format!("Regexp.last_match({num})"),
            &format!("${num}"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PerlBackrefs, "cops/style/perl_backrefs");
}
