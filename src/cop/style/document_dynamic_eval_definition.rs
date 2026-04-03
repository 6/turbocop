use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use regex::Regex;

const EVAL_METHODS: &[&str] = &["eval", "class_eval", "module_eval", "instance_eval"];

/// RuboCop only checks `eval`, `class_eval`, `module_eval`, and `instance_eval`
/// here. The main FN cluster came from bare `eval(...)` calls being omitted, and
/// the previous comment heuristic accepted any `#` in the string span instead of
/// requiring comment docs on each interpolation line or a matching heredoc block.
pub struct DocumentDynamicEvalDefinition;

impl Cop for DocumentDynamicEvalDefinition {
    fn name(&self) -> &'static str {
        "Style/DocumentDynamicEvalDefinition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(call) => call,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if !EVAL_METHODS.contains(&method_name) {
            return;
        }

        let args = match call.arguments() {
            Some(args) => args,
            None => return,
        };
        let first_arg = match args.arguments().iter().next() {
            Some(arg) => arg,
            None => return,
        };
        let interp = match first_arg.as_interpolated_string_node() {
            Some(interp) if has_interpolation(&interp) => interp,
            _ => return,
        };

        if inline_comment_docs(source, &interp) {
            return;
        }

        if is_heredoc_interpolated_string(&interp)
            && comment_block_docs(source, parse_result, &call, &interp)
        {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Add a comment block showing its appearance if interpolated.".to_string(),
        ));
    }
}

fn has_interpolation(interp: &ruby_prism::InterpolatedStringNode<'_>) -> bool {
    interp
        .parts()
        .iter()
        .any(|part| part.as_embedded_statements_node().is_some())
}

fn inline_comment_docs(
    source: &SourceFile,
    interp: &ruby_prism::InterpolatedStringNode<'_>,
) -> bool {
    let mut saw_interpolation = false;

    for part in interp.parts().iter() {
        let Some(embedded) = part.as_embedded_statements_node() else {
            continue;
        };
        saw_interpolation = true;

        let line = source
            .offset_to_line_col(embedded.location().start_offset())
            .0;
        let Some(source_line) = source_line(source, line) else {
            return false;
        };
        if !line_has_comment_doc(source_line) {
            return false;
        }
    }

    saw_interpolation
}

fn comment_block_docs(
    source: &SourceFile,
    parse_result: &ruby_prism::ParseResult<'_>,
    call: &ruby_prism::CallNode<'_>,
    interp: &ruby_prism::InterpolatedStringNode<'_>,
) -> bool {
    let body_span = heredoc_body_line_span(source, interp);
    let mut comments = heredoc_comment_blocks(source, body_span);
    comments.extend(preceding_comment_blocks(
        source,
        parse_result,
        call,
        body_span,
    ));

    if comments.is_empty() {
        return false;
    }

    let Some(regexp) = comment_regexp(source, interp) else {
        return false;
    };

    comments.iter().any(|comment| regexp.is_match(comment)) || regexp.is_match(&comments.join(""))
}

fn heredoc_body_line_span(
    source: &SourceFile,
    interp: &ruby_prism::InterpolatedStringNode<'_>,
) -> Option<(usize, usize)> {
    let mut parts = interp.parts().iter();
    let first = parts.next()?;
    let start_line = source.offset_to_line_col(first.location().start_offset()).0;

    let end_line = if let Some(closing) = interp.closing_loc() {
        let closing_line = source.offset_to_line_col(closing.start_offset()).0;
        closing_line.saturating_sub(1)
    } else {
        let last = interp.parts().iter().last()?;
        let end_offset = last.location().end_offset().saturating_sub(1);
        source.offset_to_line_col(end_offset).0
    };

    (start_line <= end_line).then_some((start_line, end_line))
}

fn heredoc_comment_blocks(source: &SourceFile, body_span: Option<(usize, usize)>) -> Vec<String> {
    let Some((start_line, end_line)) = body_span else {
        return Vec::new();
    };

    let mut blocks = Vec::new();
    for line_no in start_line..=end_line {
        let Some(line) = source_line(source, line_no) else {
            continue;
        };
        merge_adjacent_comment_line(&mut blocks, line_no, line);
    }

    blocks.into_iter().map(|(_, text)| text).collect()
}

fn preceding_comment_blocks(
    source: &SourceFile,
    parse_result: &ruby_prism::ParseResult<'_>,
    call: &ruby_prism::CallNode<'_>,
    heredoc_body_span: Option<(usize, usize)>,
) -> Vec<String> {
    let call_loc = call.location();
    let start_line = source.offset_to_line_col(call_loc.start_offset()).0;
    let end_offset = call_loc
        .end_offset()
        .saturating_sub(1)
        .max(call_loc.start_offset());
    let end_line = source.offset_to_line_col(end_offset).0;

    let mut blocks = Vec::new();
    for comment in parse_result.comments() {
        let loc = comment.location();
        let line = source.offset_to_line_col(loc.start_offset()).0;
        if line < start_line || line > end_line {
            continue;
        }
        if heredoc_body_span
            .is_some_and(|(body_start, body_end)| (body_start..=body_end).contains(&line))
        {
            continue;
        }
        let Some(text) = source.try_byte_slice(loc.start_offset(), loc.end_offset()) else {
            continue;
        };
        merge_adjacent_comment_line(&mut blocks, line, text);
    }

    blocks.into_iter().map(|(_, text)| text).collect()
}

fn merge_adjacent_comment_line(blocks: &mut Vec<(usize, String)>, line_no: usize, line: &str) {
    let Some(stripped) = strip_block_comment_prefix(line) else {
        return;
    };

    if let Some((last_line, block)) = blocks.last_mut() {
        if *last_line + 1 == line_no {
            block.push('\n');
            block.push_str(stripped);
            *last_line = line_no;
            return;
        }
    }

    blocks.push((line_no, stripped.to_string()));
}

fn strip_block_comment_prefix(line: &str) -> Option<&str> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let trimmed = line.trim_start_matches(char::is_whitespace);
    let stripped = trimmed.strip_prefix('#')?;
    if stripped.starts_with('{') {
        return None;
    }
    Some(stripped)
}

fn comment_regexp(
    source: &SourceFile,
    interp: &ruby_prism::InterpolatedStringNode<'_>,
) -> Option<Regex> {
    let pattern = comment_pattern(source, interp);
    if pattern.is_empty() {
        return None;
    }
    Regex::new(&pattern).ok()
}

fn comment_pattern(source: &SourceFile, interp: &ruby_prism::InterpolatedStringNode<'_>) -> String {
    let mut pattern = String::new();

    for part in interp.parts().iter() {
        if part.as_embedded_statements_node().is_some() {
            pattern.push_str(".+");
            continue;
        }

        if let Some(nested) = part.as_interpolated_string_node() {
            pattern.push_str(&comment_pattern(source, &nested));
            continue;
        }

        if let Some(string_part) = part.as_string_node() {
            let loc = string_part.content_loc();
            if let Some(part_source) = source.try_byte_slice(loc.start_offset(), loc.end_offset()) {
                append_source_patterns(&mut pattern, part_source);
            }
            continue;
        }

        let loc = part.location();
        if let Some(part_source) = source.try_byte_slice(loc.start_offset(), loc.end_offset()) {
            append_source_patterns(&mut pattern, part_source);
        }
    }

    pattern
}

fn append_source_patterns(pattern: &mut String, source: &str) {
    for segment in split_preserving_newlines(source) {
        pattern.push_str(&source_to_pattern(segment));
    }
}

fn split_preserving_newlines(source: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;

    for (index, ch) in source.char_indices() {
        if ch == '\n' {
            segments.push(&source[start..index + 1]);
            start = index + 1;
        }
    }

    if start < source.len() {
        segments.push(&source[start..]);
    }

    if segments.is_empty() {
        segments.push(source);
    }

    segments
}

fn source_to_pattern(source: &str) -> String {
    let source = source.strip_suffix('\r').unwrap_or(source);
    if source.trim().is_empty() {
        return r"\s+".to_string();
    }

    let stripped = strip_inline_comment_docs(source);
    if stripped.trim().is_empty() {
        return String::new();
    }

    format!(r"\s*{}", regex::escape(stripped.trim()))
}

fn strip_inline_comment_docs(source: &str) -> String {
    let mut lines = Vec::new();

    for raw_line in source.split('\n') {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if let Some(hash_index) = comment_hash_index(line) {
            lines.push(
                line[..hash_index]
                    .trim_end_matches(char::is_whitespace)
                    .to_string(),
            );
        } else {
            lines.push(line.to_string());
        }
    }

    lines.join("\n")
}

fn line_has_comment_doc(line: &str) -> bool {
    comment_hash_index(line).is_some()
}

fn comment_hash_index(line: &str) -> Option<usize> {
    line.char_indices().find_map(|(index, ch)| {
        if ch != '#' {
            return None;
        }
        let next = line[index + ch.len_utf8()..].chars().next();
        (next != Some('{')).then_some(index)
    })
}

fn is_heredoc_interpolated_string(interp: &ruby_prism::InterpolatedStringNode<'_>) -> bool {
    interp
        .opening_loc()
        .is_some_and(|opening| opening.as_slice().starts_with(b"<<"))
}

fn source_line(source: &SourceFile, line_no: usize) -> Option<&str> {
    let bytes = source.lines().nth(line_no.checked_sub(1)?)?;
    let line = std::str::from_utf8(bytes).ok()?;
    Some(line.strip_suffix('\r').unwrap_or(line))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DocumentDynamicEvalDefinition,
        "cops/style/document_dynamic_eval_definition"
    );
}
