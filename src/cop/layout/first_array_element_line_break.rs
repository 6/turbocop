use crate::cop::shared::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=7, FN=47.
///
/// The dominant FN family was implicit RHS arrays such as
/// `config.cache_store = :redis_cache_store, { ... }`. The old implementation
/// returned immediately when Prism exposed no `opening_loc()`, so it never
/// reached RuboCop's "assignment on same line" path.
///
/// The sampled FP came from deciding "multiline" by comparing the opening line
/// with the closing bracket line. RuboCop instead reasons about the element
/// lines themselves. That means arrays like `[{ type: :forge },\n]`, single
/// heredoc elements, and one-element `%w{ alpha\n}` should be accepted even
/// though the closing delimiter appears later.
///
/// This pass mirrors RuboCop's line-break check more closely: use the array's
/// start line plus the child element lines, support implicit arrays behind the
/// `AllowImplicitArrayLiterals` gate, and honor
/// `AllowMultilineFinalElement`.
pub struct FirstArrayElementLineBreak;

impl Cop for FirstArrayElementLineBreak {
    fn name(&self) -> &'static str {
        "Layout/FirstArrayElementLineBreak"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_implicit = config.get_bool("AllowImplicitArrayLiterals", false);
        let allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        let array = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let elements: Vec<ruby_prism::Node<'_>> = array.elements().iter().collect();
        if elements.is_empty() {
            return;
        }

        if array.opening_loc().is_none()
            && (allow_implicit || !assignment_on_same_line(source, array.location().start_offset()))
        {
            return;
        }

        let (start_line, _) = source.offset_to_line_col(array.location().start_offset());
        let first = first_by_line(source, &elements);
        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());
        if first_line != start_line {
            return;
        }

        let last_line = last_line(source, &elements, allow_multiline_final);
        if start_line != last_line {
            diagnostics.push(self.diagnostic(
                source,
                first_line,
                first_col,
                "Add a line break before the first element of a multi-line array.".to_string(),
            ));
        }
    }
}

fn first_by_line<'a>(
    source: &SourceFile,
    nodes: &'a [ruby_prism::Node<'a>],
) -> &'a ruby_prism::Node<'a> {
    nodes
        .iter()
        .min_by_key(|node| source.offset_to_line_col(node.location().start_offset()).0)
        .expect("nodes is non-empty")
}

fn last_line(source: &SourceFile, nodes: &[ruby_prism::Node<'_>], ignore_last: bool) -> usize {
    nodes
        .iter()
        .map(|node| {
            if ignore_last {
                source.offset_to_line_col(node.location().start_offset()).0
            } else {
                let loc = node.location();
                let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
                source.offset_to_line_col(end_offset).0
            }
        })
        .max()
        .unwrap_or(0)
}

fn assignment_on_same_line(source: &SourceFile, start_offset: usize) -> bool {
    let (line, _) = source.offset_to_line_col(start_offset);
    let Some(line_start) = source.line_col_to_offset(line, 0) else {
        return false;
    };
    let bytes = source.as_bytes();
    let Some(prefix) = bytes.get(line_start..start_offset) else {
        return false;
    };

    let mut idx = prefix.len();
    while idx > 0 && matches!(prefix[idx - 1], b' ' | b'\t') {
        idx -= 1;
    }
    if idx == 0 || prefix[idx - 1] != b'=' {
        return false;
    }
    if idx >= 2 && matches!(prefix[idx - 2], b'=' | b'!' | b'<' | b'>') {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        FirstArrayElementLineBreak,
        "cops/layout/first_array_element_line_break"
    );

    #[test]
    fn flags_implicit_arrays_on_assignment_lines_by_default() {
        let diags = run_cop_full(
            &FirstArrayElementLineBreak,
            b"options = :cache_store, {\n  expires_in: 5\n}\n",
        );

        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
    }

    #[test]
    fn allow_implicit_array_literals_skips_assignment_rhs() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowImplicitArrayLiterals".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };

        let diags = run_cop_full_with_config(
            &FirstArrayElementLineBreak,
            b"options = :cache_store, {\n  expires_in: 5\n}\n",
            config,
        );

        assert!(diags.is_empty());
    }

    #[test]
    fn allow_multiline_final_element_ignores_multiline_last_value() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultilineFinalElement".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };

        let diags =
            run_cop_full_with_config(&FirstArrayElementLineBreak, b"[a, {\n  b: c\n}]\n", config);

        assert!(diags.is_empty());
    }
}
