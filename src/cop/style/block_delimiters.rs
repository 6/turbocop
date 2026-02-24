use crate::cop::{CodeMap, Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;

pub struct BlockDelimiters;

impl Cop for BlockDelimiters {
    fn name(&self) -> &'static str {
        "Style/BlockDelimiters"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "line_count_based");
        let _procedural_methods = config.get_string_array("ProceduralMethods");
        let _functional_methods = config.get_string_array("FunctionalMethods");
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let _allow_braces_on_procedural =
            config.get_bool("AllowBracesOnProceduralOneLiners", false);
        let braces_required_methods = config.get_string_array("BracesRequiredMethods");

        if enforced_style != "line_count_based" {
            return;
        }

        let allowed = allowed_methods
            .unwrap_or_else(|| vec!["lambda".to_string(), "proc".to_string(), "it".to_string()]);
        let patterns = allowed_patterns.unwrap_or_default();
        let braces_required = braces_required_methods.unwrap_or_default();

        let mut visitor = BlockDelimitersVisitor {
            source,
            cop: self,
            diagnostics: Vec::new(),
            ignored_blocks: HashSet::new(),
            allowed_methods: allowed,
            allowed_patterns: patterns,
            braces_required_methods: braces_required,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct BlockDelimitersVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a BlockDelimiters,
    diagnostics: Vec<Diagnostic>,
    ignored_blocks: HashSet<usize>,
    allowed_methods: Vec<String>,
    allowed_patterns: Vec<String>,
    braces_required_methods: Vec<String>,
}

impl<'a> BlockDelimitersVisitor<'a> {
    fn check_block(&mut self, block_node: &ruby_prism::BlockNode<'_>, method_name: &[u8]) {
        let method_str = std::str::from_utf8(method_name).unwrap_or("");

        // Skip AllowedMethods (default: lambda, proc, it)
        if self.allowed_methods.iter().any(|m| m == method_str) {
            return;
        }

        // Skip AllowedPatterns
        for pattern in &self.allowed_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if re.is_match(method_str) {
                    return;
                }
            }
        }

        let opening_loc = block_node.opening_loc();
        let closing_loc = block_node.closing_loc();
        let opening = opening_loc.as_slice();

        let (open_line, _) = self.source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, _) = self.source.offset_to_line_col(closing_loc.start_offset());
        let is_single_line = open_line == close_line;

        // BracesRequiredMethods: must use braces
        if self.braces_required_methods.iter().any(|m| m == method_str) {
            if opening == b"do" {
                let (line, column) = self.source.offset_to_line_col(opening_loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Brace delimiters `{{...}}` required for '{}' method.",
                        method_str
                    ),
                ));
            }
            return;
        }

        // require_do_end: single-line do-end blocks with rescue/ensure clauses
        // cannot be converted to braces (syntax error). Skip these.
        if is_single_line && opening == b"do" && block_has_rescue_or_ensure(block_node) {
            return;
        }

        // line_count_based style
        if is_single_line && opening == b"do" {
            let (line, column) = self.source.offset_to_line_col(opening_loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Prefer `{...}` over `do...end` for single-line blocks.".to_string(),
            ));
        } else if !is_single_line && opening == b"{" {
            let (line, column) = self.source.offset_to_line_col(opening_loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Prefer `do...end` over `{...}` for multi-line blocks.".to_string(),
            ));
        }
    }
}

impl<'a> Visit<'_> for BlockDelimitersVisitor<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        // Phase 1: For non-parenthesized calls with arguments, mark argument blocks
        // as ignored. Changing delimiters on these blocks would change binding
        // semantics (braces bind tighter than do..end).
        let is_parenthesized = node.opening_loc().is_some();
        let method_name = node.name().as_slice();
        let is_assignment = method_name.ends_with(b"=")
            && method_name != b"=="
            && method_name != b"!="
            && method_name != b"<="
            && method_name != b">="
            && method_name != b"===";

        if !is_parenthesized && !is_assignment {
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    collect_ignored_blocks(&arg, &mut self.ignored_blocks);
                }
            }
        }

        // Phase 2: Check this call's block (if any)
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                let offset = block_node.opening_loc().start_offset();
                if !self.ignored_blocks.contains(&offset) {
                    self.check_block(&block_node, method_name);
                }
            }
        }

        // Recurse into children
        ruby_prism::visit_call_node(self, node);
    }
}

/// Check if a block's body contains rescue or ensure clauses.
/// In Prism, this manifests as a BeginNode body with rescue_clause or ensure_clause.
fn block_has_rescue_or_ensure(block_node: &ruby_prism::BlockNode<'_>) -> bool {
    if let Some(body) = block_node.body() {
        if let Some(begin_node) = body.as_begin_node() {
            return begin_node.rescue_clause().is_some() || begin_node.ensure_clause().is_some();
        }
    }
    false
}

/// Recursively collect blocks inside argument expressions of non-parenthesized
/// method calls. These blocks must be ignored because changing `{...}` to
/// `do...end` (or vice versa) would change block binding.
fn collect_ignored_blocks(node: &ruby_prism::Node<'_>, ignored: &mut HashSet<usize>) {
    // CallNode: mark its block as ignored, recurse into receiver + arguments
    if let Some(call) = node.as_call_node() {
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                ignored.insert(block_node.opening_loc().start_offset());
            }
        }
        if let Some(receiver) = call.receiver() {
            collect_ignored_blocks(&receiver, ignored);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                collect_ignored_blocks(&arg, ignored);
            }
        }
        return;
    }

    // KeywordHashNode (unbraced hash in argument position)
    if let Some(kwh) = node.as_keyword_hash_node() {
        for element in kwh.elements().iter() {
            collect_ignored_blocks(&element, ignored);
        }
        return;
    }

    // HashNode (braced hash) — skip per vendor logic (braces prevent rebinding)
    if node.as_hash_node().is_some() {
        return;
    }

    // AssocNode (key: value pair)
    if let Some(assoc) = node.as_assoc_node() {
        collect_ignored_blocks(&assoc.value(), ignored);
        return;
    }

    // AssocSplatNode (**hash)
    if let Some(splat) = node.as_assoc_splat_node() {
        if let Some(value) = splat.value() {
            collect_ignored_blocks(&value, ignored);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BlockDelimiters, "cops/style/block_delimiters");

    #[test]
    fn no_offense_proc_in_keyword_arg() {
        // Proc block in keyword arg without parens — changing braces would change semantics
        let source = b"my_method :arg1, arg2: proc {\n  something\n}, arg3: :another_value\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag proc block in keyword argument position, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_safe_navigation_non_parenthesized() {
        // Safe-navigation call with non-parenthesized block arg
        let source = b"foo&.bar baz {\n  y\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag block in safe-navigation non-parenthesized call, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_chained_method_block_in_arg() {
        // Block result chained and used as argument
        let source = b"foo bar + baz {\n}.qux.quux\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag chained block in non-parenthesized arg, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_lambda_in_keyword_arg_without_parens() {
        // lambda block in keyword arg of non-parenthesized call
        let source = b"foo :bar, :baz, qux: lambda { |a|\n  bar a\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag lambda block in keyword arg, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_do_end_single_line_rescue_array() {
        // Single-line do-end with rescue that has array exception type
        // This needs do-end because {} + rescue + array creates ambiguity
        let source = b"foo do next unless bar; rescue StandardError; end\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag single-line do-end with rescue+semicolon, got: {:?}",
            diags
        );
    }
}
