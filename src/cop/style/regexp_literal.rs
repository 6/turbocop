use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// FP fix (2026-03): slashes inside `#{}` interpolation segments were wrongly
/// counted as inner slashes, causing false "Use %r" suggestions on regexps like
/// `/#{Regexp.quote("</")}/ `. RuboCop's `node_body` only examines `:str` children,
/// so interpolation content is excluded. Fixed by iterating over Prism's `parts()`
/// and only collecting `StringNode` content for the slash check.
///
/// FP fix (2026-04): RuboCop checks the parser-visible regexp `content` for the
/// leading space/`=` exemption, and that content skips `#{...}` interpolation
/// nodes. As a result, method arguments like
/// `assert_match %r(#{attribute}="#{value}")` and
/// `with(%r{#{Regexp.escape(duration.to_s)} seconds})` are accepted because
/// their first literal string chunk begins with `=` or a space. Standalone
/// interpolated `%r` literals must still be flagged, so the exemption remains
/// limited to direct call-like arguments while switching the prefix check to
/// the string-part content RuboCop uses.
pub struct RegexpLiteral;

impl Cop for RegexpLiteral {
    fn name(&self) -> &'static str {
        "Style/RegexpLiteral"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = RegexpLiteralVisitor {
            cop: self,
            source,
            config,
            diagnostics,
            ancestors: Vec::new(),
        };
        visitor.visit(&parse_result.node());
    }
}

struct RegexpLiteralVisitor<'a, 'pr> {
    cop: &'a RegexpLiteral,
    source: &'a SourceFile,
    config: &'a CopConfig,
    diagnostics: &'a mut Vec<Diagnostic>,
    ancestors: Vec<ruby_prism::Node<'pr>>,
}

impl<'pr> RegexpLiteralVisitor<'_, 'pr> {
    fn visit_candidate(&mut self, node: &ruby_prism::Node<'pr>) {
        let enforced_style = self.config.get_str("EnforcedStyle", "slashes");
        let allow_inner_slashes = self.config.get_bool("AllowInnerSlashes", false);

        let (open_bytes, content_bytes, node_start, node_end): (Vec<u8>, Vec<u8>, usize, usize) =
            if let Some(re) = node.as_regular_expression_node() {
                let opening = re.opening_loc();
                let content = re.content_loc().as_slice();
                let loc = re.location();
                (
                    opening.as_slice().to_vec(),
                    content.to_vec(),
                    loc.start_offset(),
                    loc.end_offset(),
                )
            } else if let Some(re) = node.as_interpolated_regular_expression_node() {
                let opening = re.opening_loc();
                let loc = re.location();
                let mut content = Vec::new();

                // Only collect content from string literal parts, skipping interpolation.
                // RuboCop's `node_body` only examines `:str` children, so slashes
                // inside `#{}` interpolation are not counted as inner slashes.
                for part in re.parts().iter() {
                    if let Some(s) = part.as_string_node() {
                        content.extend_from_slice(s.location().as_slice());
                    }
                }

                (
                    opening.as_slice().to_vec(),
                    content,
                    loc.start_offset(),
                    loc.end_offset(),
                )
            } else {
                return;
            };

        let is_slash = open_bytes == b"/";
        let is_percent_r = open_bytes.starts_with(b"%r");
        let has_slash = content_bytes.contains(&b'/');
        let is_multiline = {
            let (start_line, _) = self.source.offset_to_line_col(node_start);
            let (end_line, _) = self.source.offset_to_line_col(node_end);
            end_line > start_line
        };

        // %r with content starting with space or = may be used to avoid syntax errors
        // when the regexp is a direct call argument:
        //   do_something %r{ regexp}  # valid
        //   do_something / regexp/    # syntax error
        let content_starts_with_space_or_eq =
            !content_bytes.is_empty() && (content_bytes[0] == b' ' || content_bytes[0] == b'=');
        let allowed_percent_r_call_argument =
            content_starts_with_space_or_eq && self.direct_call_like_argument(node_start, node_end);

        match enforced_style {
            "slashes" => {
                if is_percent_r {
                    if has_slash && !allow_inner_slashes {
                        return;
                    }
                    if allowed_percent_r_call_argument {
                        return;
                    }
                    self.add_offense(node_start, "Use `//` around regular expression.");
                }
            }
            "percent_r" => {
                if is_slash {
                    self.add_offense(node_start, "Use `%r` around regular expression.");
                }
            }
            "mixed" => {
                if is_multiline {
                    if is_slash {
                        self.add_offense(node_start, "Use `%r` around regular expression.");
                    }
                } else if is_percent_r {
                    if has_slash && !allow_inner_slashes {
                        return;
                    }
                    if allowed_percent_r_call_argument {
                        return;
                    }
                    self.add_offense(node_start, "Use `//` around regular expression.");
                }
            }
            _ => {}
        }

        if enforced_style == "slashes" && is_slash && has_slash && !allow_inner_slashes {
            self.add_offense(node_start, "Use `%r` around regular expression.");
        }
    }

    fn direct_call_like_argument(&self, node_start: usize, node_end: usize) -> bool {
        let Some(parent) = self.ancestors.last() else {
            return false;
        };

        if let Some(call) = parent.as_call_node() {
            return call.arguments().is_some_and(|args| {
                args.arguments().iter().any(|arg| {
                    let loc = arg.location();
                    loc.start_offset() == node_start && loc.end_offset() == node_end
                })
            });
        }

        if let Some(super_node) = parent.as_super_node() {
            return super_node.arguments().is_some_and(|args| {
                args.arguments().iter().any(|arg| {
                    let loc = arg.location();
                    loc.start_offset() == node_start && loc.end_offset() == node_end
                })
            });
        }

        if let Some(yield_node) = parent.as_yield_node() {
            return yield_node.arguments().is_some_and(|args| {
                args.arguments().iter().any(|arg| {
                    let loc = arg.location();
                    loc.start_offset() == node_start && loc.end_offset() == node_end
                })
            });
        }

        if parent.as_arguments_node().is_some() {
            return self
                .ancestors
                .get(self.ancestors.len().saturating_sub(2))
                .is_some_and(|grandparent| {
                    grandparent.as_call_node().is_some()
                        || grandparent.as_super_node().is_some()
                        || grandparent.as_yield_node().is_some()
                });
        }

        false
    }

    fn add_offense(&mut self, start_offset: usize, message: &str) {
        let (line, column) = self.source.offset_to_line_col(start_offset);
        self.diagnostics.push(
            self.cop
                .diagnostic(self.source, line, column, message.to_string()),
        );
    }
}

impl<'pr> Visit<'pr> for RegexpLiteralVisitor<'_, 'pr> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.visit_candidate(&node);
        self.ancestors.push(node);
    }

    fn visit_branch_node_leave(&mut self) {
        self.ancestors.pop();
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.visit_candidate(&node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RegexpLiteral, "cops/style/regexp_literal");
}
