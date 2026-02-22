use ruby_prism::Visit;

use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FirstHashElementIndentation;

impl Cop for FirstHashElementIndentation {
    fn name(&self) -> &'static str {
        "Layout/FirstHashElementIndentation"
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
        let style = config.get_str("EnforcedStyle", "special_inside_parentheses");
        let width = config.get_usize("IndentationWidth", 2);
        let mut visitor = HashIndentVisitor {
            cop: self,
            source,
            style,
            width,
            diagnostics: Vec::new(),
            handled_hashes: Vec::new(),
            parent_pair_col: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct HashIndentVisitor<'a> {
    cop: &'a FirstHashElementIndentation,
    source: &'a SourceFile,
    style: &'a str,
    width: usize,
    diagnostics: Vec<Diagnostic>,
    /// Start offsets of hash nodes already checked via a parent call with parentheses.
    handled_hashes: Vec<usize>,
    /// When visiting a hash that is a value in a pair (AssocNode), this stores
    /// the pair's column and whether a right sibling begins on a subsequent line.
    parent_pair_col: Option<usize>,
}

impl HashIndentVisitor<'_> {
    fn check_hash(&mut self, hash_node: &ruby_prism::HashNode<'_>, left_paren_col: Option<usize>) {
        let opening_loc = hash_node.opening_loc();
        if opening_loc.as_slice() != b"{" {
            return;
        }

        let elements: Vec<_> = hash_node.elements().iter().collect();
        if elements.is_empty() {
            return;
        }

        let first_element = &elements[0];
        let (open_line, _) = self.source.offset_to_line_col(opening_loc.start_offset());
        let first_loc = first_element.location();
        let (elem_line, elem_col) = self.source.offset_to_line_col(first_loc.start_offset());

        if elem_line == open_line {
            return;
        }

        let open_line_bytes = self.source.lines().nth(open_line - 1).unwrap_or(b"");
        let open_line_indent = indentation_of(open_line_bytes);
        let (_, open_col) = self.source.offset_to_line_col(opening_loc.start_offset());

        let expected = match self.style {
            "consistent" => open_line_indent + self.width,
            "align_braces" => open_col + self.width,
            _ => {
                // RuboCop's indent_base priority for special_inside_parentheses:
                // 1. If parent is a pair (hash key/value) where key and value start
                //    on the same line and the pair has a right sibling on a
                //    subsequent line, use the pair's column.
                // 2. If inside parenthesized method call, use paren_col + 1.
                // 3. Fall back to line indentation.
                if let Some(pair_col) = self.parent_pair_col {
                    pair_col + self.width
                } else if let Some(paren_col) = left_paren_col {
                    paren_col + 1 + self.width
                } else {
                    open_line_indent + self.width
                }
            }
        };

        if elem_col != expected {
            let base_indent = if left_paren_col.is_some()
                && self.style != "consistent"
                && self.style != "align_braces"
            {
                left_paren_col.unwrap() + 1
            } else {
                open_line_indent
            };
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                elem_line,
                elem_col,
                format!(
                    "Use {} (not {}) spaces for indentation of the first element.",
                    self.width,
                    elem_col.saturating_sub(base_indent)
                ),
            ));
        }
    }

    fn find_hash_args_in_call(
        &mut self,
        node: &ruby_prism::Node<'_>,
        paren_line: usize,
        paren_col: usize,
    ) {
        if let Some(hash) = node.as_hash_node() {
            let opening_loc = hash.opening_loc();
            if opening_loc.as_slice() == b"{" {
                let (brace_line, _) = self.source.offset_to_line_col(opening_loc.start_offset());
                if brace_line == paren_line {
                    self.handled_hashes.push(hash.location().start_offset());
                    self.check_hash(&hash, Some(paren_col));
                }
            }
            for elem in hash.elements().iter() {
                self.find_hash_args_in_call(&elem, paren_line, paren_col);
            }
            return;
        }

        if node.as_call_node().is_some() {
            return;
        }

        if let Some(kw_hash) = node.as_keyword_hash_node() {
            for elem in kw_hash.elements().iter() {
                self.find_hash_args_in_call(&elem, paren_line, paren_col);
            }
            return;
        }

        if let Some(assoc) = node.as_assoc_node() {
            self.find_hash_args_in_call(&assoc.value(), paren_line, paren_col);
            return;
        }

        if let Some(splat) = node.as_splat_node() {
            if let Some(expr) = splat.expression() {
                self.find_hash_args_in_call(&expr, paren_line, paren_col);
            }
            return;
        }

        if let Some(parens) = node.as_parentheses_node() {
            if let Some(body) = parens.body() {
                self.find_hash_args_in_call(&body, paren_line, paren_col);
            }
            return;
        }

        if let Some(array) = node.as_array_node() {
            for elem in array.elements().iter() {
                self.find_hash_args_in_call(&elem, paren_line, paren_col);
            }
        }
    }
}

impl HashIndentVisitor<'_> {
    /// For each pair element whose value is a HashNode starting with `{`,
    /// check RuboCop's parent_hash_key indentation condition: if the pair's
    /// key and value start on the same line AND the pair has a right sibling
    /// on a subsequent line, set `parent_pair_col` so the child hash uses
    /// the pair's column as indent base.
    fn visit_pairs_with_hash_values(&mut self, elements: ruby_prism::NodeList<'_>) {
        let elems: Vec<_> = elements.iter().collect();
        for (i, elem) in elems.iter().enumerate() {
            let assoc = match elem.as_assoc_node() {
                Some(a) => a,
                None => {
                    self.visit(elem);
                    continue;
                }
            };

            // Check if the value is a HashNode with `{`
            let value = assoc.value();
            let is_hash_value = value
                .as_hash_node()
                .is_some_and(|h| h.opening_loc().as_slice() == b"{");

            if !is_hash_value || self.style == "consistent" || self.style == "align_braces" {
                self.visit(elem);
                continue;
            }

            // Check condition: key and value begin on the same line
            let (key_line, _) = self
                .source
                .offset_to_line_col(assoc.key().location().start_offset());
            let (val_line, _) = self
                .source
                .offset_to_line_col(value.location().start_offset());
            if key_line != val_line {
                self.visit(elem);
                continue;
            }

            // Check condition: right sibling begins on a subsequent line
            let has_right_sibling_on_next_line = if i + 1 < elems.len() {
                let (pair_last_line, _) =
                    self.source.offset_to_line_col(elem.location().end_offset());
                let (sibling_line, _) = self
                    .source
                    .offset_to_line_col(elems[i + 1].location().start_offset());
                pair_last_line < sibling_line
            } else {
                false
            };

            if has_right_sibling_on_next_line {
                let (_, pair_col) = self
                    .source
                    .offset_to_line_col(elem.location().start_offset());
                let saved = self.parent_pair_col;
                self.parent_pair_col = Some(pair_col);
                self.visit(elem);
                self.parent_pair_col = saved;
            } else {
                self.visit(elem);
            }
        }
    }
}

impl Visit<'_> for HashIndentVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        if let Some(open_paren_loc) = node.opening_loc() {
            if open_paren_loc.as_slice() == b"(" {
                let (paren_line, paren_col) = self
                    .source
                    .offset_to_line_col(open_paren_loc.start_offset());
                if let Some(args) = node.arguments() {
                    for arg in args.arguments().iter() {
                        self.find_hash_args_in_call(&arg, paren_line, paren_col);
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'_>) {
        let offset = node.location().start_offset();
        if !self.handled_hashes.contains(&offset) {
            self.check_hash(node, None);
        }
        // Clear parent_pair_col after the immediate hash uses it,
        // so that nested hashes inside this one don't inherit it.
        let saved = self.parent_pair_col;
        self.parent_pair_col = None;
        // Before visiting children, check if any element is a pair whose value
        // is a hash. If so, set parent_pair_col for that child hash.
        self.visit_pairs_with_hash_values(node.elements());
        self.parent_pair_col = saved;
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'_>) {
        // keyword hashes can also contain pairs whose values are hashes
        self.visit_pairs_with_hash_values(node.elements());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        FirstHashElementIndentation,
        "cops/layout/first_hash_element_indentation"
    );

    #[test]
    fn same_line_elements_ignored() {
        let source = b"x = { a: 1, b: 2 }\n";
        let diags = run_cop_full(&FirstHashElementIndentation, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn align_braces_style() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("align_braces".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"x = {\n      a: 1\n}\n";
        let diags = run_cop_full_with_config(&FirstHashElementIndentation, src, config.clone());
        assert!(
            diags.is_empty(),
            "align_braces should accept element at brace column + width"
        );

        let src2 = b"x = {\n  a: 1\n}\n";
        let diags2 = run_cop_full_with_config(&FirstHashElementIndentation, src2, config);
        assert_eq!(
            diags2.len(),
            1,
            "align_braces should flag element not at brace column + width"
        );
    }

    #[test]
    fn special_inside_parentheses_method_call() {
        let source = b"func({\n       a: 1\n     })\n";
        let diags = run_cop_full(&FirstHashElementIndentation, source);
        assert!(
            diags.is_empty(),
            "should accept special indentation inside parentheses"
        );
    }

    #[test]
    fn special_inside_parentheses_flags_consistent_indent() {
        let source = b"func({\n  a: 1\n})\n";
        let diags = run_cop_full(&FirstHashElementIndentation, source);
        assert_eq!(
            diags.len(),
            1,
            "should flag consistent indentation inside parentheses"
        );
    }

    #[test]
    fn special_inside_parentheses_with_second_arg() {
        let source = b"func(x, {\n       a: 1\n     })\n";
        let diags = run_cop_full(&FirstHashElementIndentation, source);
        assert!(
            diags.is_empty(),
            "should accept special indentation for second hash arg"
        );
    }

    #[test]
    fn brace_not_on_same_line_as_paren_uses_line_indent() {
        let source = b"func(\n  {\n    a: 1\n  }\n)\n";
        let diags = run_cop_full(&FirstHashElementIndentation, source);
        assert!(
            diags.is_empty(),
            "brace on different line from paren should use line indent"
        );
    }

    #[test]
    fn safe_navigation_with_hash_arg() {
        let source = b"receiver&.func({\n                 a: 1\n               })\n";
        let diags = run_cop_full(&FirstHashElementIndentation, source);
        assert!(
            diags.is_empty(),
            "should handle safe navigation with hash arg"
        );
    }

    #[test]
    fn index_assignment_not_treated_as_paren() {
        let source = b"    config['key'] = {\n      val: 1\n    }\n";
        let diags = run_cop_full(&FirstHashElementIndentation, source);
        assert!(
            diags.is_empty(),
            "index assignment should not use paren context"
        );
    }

    #[test]
    fn nested_hash_in_keyword_arg() {
        let source = b"Config.new('Key' => {\n             val: 1\n           })\n";
        let diags = run_cop_full(&FirstHashElementIndentation, source);
        assert!(
            diags.is_empty(),
            "nested hash in keyword arg should use paren context"
        );
    }
}
