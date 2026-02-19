use ruby_prism::Visit;

use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineMethodCallIndentation;

impl Cop for MultilineMethodCallIndentation {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodCallIndentation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let style = config.get_str("EnforcedStyle", "aligned");
        let width = config.get_usize("IndentationWidth", 2);
        let mut visitor = ChainVisitor {
            cop: self,
            source,
            style,
            width,
            diagnostics: Vec::new(),
            in_paren_args: false,
            in_hash_value: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ChainVisitor<'a> {
    cop: &'a MultilineMethodCallIndentation,
    source: &'a SourceFile,
    style: &'a str,
    width: usize,
    diagnostics: Vec<Diagnostic>,
    in_paren_args: bool,
    /// True when visiting the value side of a hash pair (AssocNode).
    /// RuboCop checks chain indentation inside hash pair values even
    /// when they're also inside parenthesized arguments.
    in_hash_value: bool,
}

impl ChainVisitor<'_> {
    fn check_call(&mut self, call_node: &ruby_prism::CallNode<'_>) {
        // Must have a receiver (chained call)
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return,
        };

        // Must have a call operator (the `.` part)
        let dot_loc = match call_node.call_operator_loc() {
            Some(loc) => loc,
            None => return,
        };

        let receiver_loc = receiver.location();
        let (recv_end_line, _) = self.source.offset_to_line_col(receiver_loc.end_offset());
        let (msg_line, msg_col) = self.source.offset_to_line_col(dot_loc.start_offset());

        // Only check when the dot is on a different line than the end of its receiver.
        if msg_line == recv_end_line {
            return;
        }

        // The dot must be a continuation dot (first non-whitespace on its line).
        if !is_continuation_dot(self.source, dot_loc.start_offset()) {
            return;
        }

        // RuboCop skips inside parenthesized call arguments, UNLESS the chain
        // is inside a hash pair value (where it applies hash-pair alignment).
        // However, if we entered NEW paren args inside a hash value (deeper
        // nesting), we still skip — matching RuboCop's `not_for_this_cop?`.
        if self.in_paren_args && !self.in_hash_value {
            return;
        }

        let expected = match self.style {
            "indented" | "indented_relative_to_receiver" => {
                let chain_start_line = find_chain_start_line(self.source, &receiver);
                // Walk backwards from the chain start line to find the first
                // non-continuation-dot line. This matches RuboCop's `left_hand_side`
                // which walks up through parent nodes to find the topmost chain.
                let base_line = find_non_continuation_ancestor_line(self.source, chain_start_line);
                let base_line_bytes = self
                    .source
                    .lines()
                    .nth(base_line - 1)
                    .unwrap_or(b"");
                let base_indent = indentation_of(base_line_bytes);
                // RuboCop adds an extra IndentationWidth when the chain is inside
                // a keyword expression like `return`, `if`, `while`, `until`, `for`.
                let kw_extra = keyword_extra_indent(self.source, call_node, self.width);
                base_indent + self.width + kw_extra
            }
            _ => {
                // "aligned" (default)
                if self.in_hash_value {
                    // Inside a hash pair value: RuboCop uses the chain root's
                    // start column as the alignment base, BUT with two escape
                    // hatches:
                    //
                    // 1. `aligned_with_first_line_dot?`: if the current dot's
                    //    column matches an inline dot on the chain's first line,
                    //    RuboCop considers it aligned and skips.
                    let chain_root_line = find_chain_start_line(self.source, &receiver);
                    if has_matching_dot_on_line(self.source, &receiver, chain_root_line, msg_col) {
                        return;
                    }
                    // 2. Block chain continuation: when the receiver is a
                    //    call-with-block (single-line block), the block-bearing
                    //    call's dot column is the alignment base.
                    if let Some(col) = find_block_chain_col(self.source, &receiver, msg_line) {
                        col
                    } else {
                        find_chain_root_col(self.source, &receiver)
                    }
                } else {
                    // Try to find a previous continuation dot to align with.
                    // Also check for block chain continuation.
                    match find_alignment_base_col(self.source, &receiver, msg_line) {
                        Some(col) => col,
                        // First multiline step with no alignment base;
                        // accept whatever position the user chose.
                        None => return,
                    }
                }
            }
        };

        if msg_col != expected {
            let msg = match self.style {
                "aligned" => {
                    // Build message matching RuboCop format
                    let selector = call_node.name().as_slice();
                    let selector_str = std::str::from_utf8(selector).unwrap_or("?");
                    let (base_name, base_line) = find_alignment_base_description(
                        self.source, &receiver,
                    );
                    format!(
                        "Align `.{selector_str}` with `{base_name}` on line {base_line}."
                    )
                }
                _ => {
                    let chain_start_line = find_chain_start_line(self.source, &receiver);
                    let base_line = find_non_continuation_ancestor_line(self.source, chain_start_line);
                    let chain_line_bytes = self
                        .source
                        .lines()
                        .nth(base_line - 1)
                        .unwrap_or(b"");
                    let chain_indent = indentation_of(chain_line_bytes);
                    format!(
                        "Use {} (not {}) spaces for indentation of a chained method call.",
                        self.width,
                        msg_col.saturating_sub(chain_indent)
                    )
                }
            };
            self.diagnostics
                .push(self.cop.diagnostic(self.source, msg_line, msg_col, msg));
        }
    }
}

impl Visit<'_> for ChainVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        // Check this call node for alignment issues
        self.check_call(node);

        // Visit receiver normally (inherits current context)
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }

        // Visit arguments: if call has parens, mark as in_paren_args
        let has_parens = node.opening_loc().is_some();
        if let Some(args) = node.arguments() {
            if has_parens {
                let saved_paren = self.in_paren_args;
                self.in_paren_args = true;
                self.visit(&args.as_node());
                self.in_paren_args = saved_paren;
            } else {
                self.visit(&args.as_node());
            }
        }

        // Visit block normally (inherits current context)
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'_>) {
        // Grouped expressions like `(foo\n  .bar)` — RuboCop skips these too
        let saved_paren = self.in_paren_args;
        self.in_paren_args = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_paren_args = saved_paren;
    }

    fn visit_assoc_node(&mut self, node: &ruby_prism::AssocNode<'_>) {
        // Visit key normally
        self.visit(&node.key());

        // Visit value with in_hash_value = true — RuboCop checks chain
        // indentation inside hash pair values even within parenthesized args.
        let saved_hash = self.in_hash_value;
        self.in_hash_value = true;
        self.visit(&node.value());
        self.in_hash_value = saved_hash;
    }
}

/// For `aligned` style: find the column of a suitable alignment base.
/// Tries (in order):
/// 1. A block chain continuation (single-line block receiver)
/// 2. A previous continuation dot in the chain on an earlier line
///
/// Block chain is checked first because when a single-line block is the
/// receiver, the alignment base should be the block-bearing call's dot,
/// not an earlier continuation dot from below the block boundary.
fn find_alignment_base_col(
    source: &SourceFile,
    receiver: &ruby_prism::Node<'_>,
    current_dot_line: usize,
) -> Option<usize> {
    // First try: block chain continuation — when the receiver is a
    // call-with-block whose dot+method+block is single-line, align with
    // that call's dot. In Prism, blocks are children of CallNode (unlike
    // Parser where BlockNode wraps the send).
    // Pattern: `foo.bar { ... }\n  .baz` → align .baz with .bar's dot
    // We check from the DOT position (not the CallNode start, which includes
    // the full receiver chain) to the end of the block.
    if let Some(call) = receiver.as_call_node() {
        if call.block().is_some() {
            if let Some(dot_loc) = call.call_operator_loc() {
                let (dot_line, dot_col) = source.offset_to_line_col(dot_loc.start_offset());
                let loc = call.location();
                let (end_line, _) = source.offset_to_line_col(loc.end_offset());
                if dot_line == end_line && dot_line < current_dot_line {
                    return Some(dot_col);
                }
            }
        }
    }

    // Second try: previous continuation dot in the chain
    if let Some(col) = find_alignment_dot_col(source, receiver, current_dot_line) {
        return Some(col);
    }

    None
}

/// Block chain alignment ONLY (no continuation dot search). Used for hash
/// pair values where continuation dot alignment is NOT wanted, but block
/// chain continuation IS.
fn find_block_chain_col(
    source: &SourceFile,
    receiver: &ruby_prism::Node<'_>,
    current_dot_line: usize,
) -> Option<usize> {
    if let Some(call) = receiver.as_call_node() {
        if call.block().is_some() {
            if let Some(dot_loc) = call.call_operator_loc() {
                let (dot_line, dot_col) = source.offset_to_line_col(dot_loc.start_offset());
                let loc = call.location();
                let (end_line, _) = source.offset_to_line_col(loc.end_offset());
                if dot_line == end_line && dot_line < current_dot_line {
                    return Some(dot_col);
                }
            }
        }
    }
    None
}

/// Find the column of the last continuation-style dot on a given line.
/// This handles block chain patterns like `foo.bar { }.baz` where we need
/// to find `.bar`'s dot column.
fn find_last_dot_on_line(source: &SourceFile, line: usize) -> Option<usize> {
    let lines: Vec<&[u8]> = source.lines().collect();
    if line == 0 || line > lines.len() {
        return None;
    }
    let line_bytes = lines[line - 1];
    // Find the first `.` that follows a word character or `}` or `)` — this is
    // an inline dot (method call). We want the column of the FIRST inline dot
    // after the receiver starts, to use as the alignment target.
    let mut last_dot_col = None;
    for (i, &b) in line_bytes.iter().enumerate() {
        if b == b'.' && i > 0 {
            let prev = line_bytes[i - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b')' || prev == b'}' || prev == b']' {
                // Check it's not `..` (range)
                if i + 1 < line_bytes.len() && line_bytes[i + 1] == b'.' {
                    continue;
                }
                last_dot_col = Some(i);
            }
        }
    }
    last_dot_col
}

/// For `aligned` style: find the column of the dot (call operator) from the
/// nearest ancestor in the chain that is on a previous line.
/// Only considers dots that are "continuation dots" — at the start of their line
/// (after whitespace). Inline dots like `foo.bar` on the same expression line
/// are not valid alignment targets.
fn find_alignment_dot_col(
    source: &SourceFile,
    receiver: &ruby_prism::Node<'_>,
    current_dot_line: usize,
) -> Option<usize> {
    // Walk up the receiver chain looking for a dot on a different (earlier) line
    if let Some(call) = receiver.as_call_node() {
        if let Some(dot_loc) = call.call_operator_loc() {
            let (dot_line, dot_col) = source.offset_to_line_col(dot_loc.start_offset());
            if dot_line < current_dot_line {
                // Found a dot on a previous line.
                // Only use it as alignment target if it's a "continuation dot"
                // (i.e., it's the first non-whitespace on its line).
                if is_continuation_dot(source, dot_loc.start_offset()) {
                    // Check if there's an even earlier continuation dot
                    if let Some(recv) = call.receiver() {
                        if let Some(earlier) =
                            find_alignment_dot_col(source, &recv, dot_line)
                        {
                            return Some(earlier);
                        }
                    }
                    return Some(dot_col);
                }
                // Dot is inline (not a continuation dot); keep looking for earlier dots
                if let Some(recv) = call.receiver() {
                    return find_alignment_dot_col(source, &recv, current_dot_line);
                }
                // No continuation dot found anywhere in the chain
                return None;
            }
            // Dot is on the same line as current; keep looking up
            if let Some(recv) = call.receiver() {
                return find_alignment_dot_col(source, &recv, current_dot_line);
            }
        }
    }

    // Handle block receivers: when receiver is a block node, look through it
    if let Some(block) = receiver.as_block_node() {
        // In Prism, a block's "call" is actually the parent — but from inside
        // the visitor we see the block as a child. Check if this block is
        // single-line and find the dot on that line.
        let block_loc = block.location();
        let (block_start_line, _) = source.offset_to_line_col(block_loc.start_offset());
        let (block_end_line, _) = source.offset_to_line_col(block_loc.end_offset());
        if block_start_line == block_end_line && block_start_line < current_dot_line {
            // Single-line block on a previous line — find last dot on that line
            if let Some(col) = find_last_dot_on_line(source, block_start_line) {
                return Some(col);
            }
        }
    }

    None
}

/// Check whether the dot at the given byte offset is the first non-whitespace
/// character on its line (a "continuation dot").
fn is_continuation_dot(source: &SourceFile, dot_offset: usize) -> bool {
    let bytes = source.as_bytes();
    let mut pos = dot_offset;
    // Walk backwards to start of line
    while pos > 0 && bytes[pos - 1] != b'\n' {
        pos -= 1;
    }
    // Check if everything between line start and dot is whitespace
    while pos < dot_offset {
        if bytes[pos] != b' ' && bytes[pos] != b'\t' {
            return false;
        }
        pos += 1;
    }
    true
}

/// RuboCop's `aligned_with_first_line_dot?`: check whether the first call
/// with a dot in the receiver chain (walking from root up) has a dot on
/// `line` at column `target_col`. Used for hash pair value alignment.
///
/// RuboCop's logic: `first_call = first_call_has_a_dot(node);
/// return false if first_call == node.receiver`. If the first call IS the
/// direct receiver (2-call chain like `receiver.where.or`), returns false.
/// If there are deeper calls (3+ chain like `template.subs.where.or`),
/// checks the first call's dot.
fn has_matching_dot_on_line(
    source: &SourceFile,
    receiver: &ruby_prism::Node<'_>,
    line: usize,
    target_col: usize,
) -> bool {
    // Find the first call with a dot in the chain (deepest call from root).
    let first_call_dot = find_first_call_dot(source, receiver);
    if let Some((fc_line, fc_col, fc_offset)) = first_call_dot {
        // Check `first_call == node.receiver`: if the first call's dot
        // belongs to the direct receiver, skip (return false).
        if let Some(call) = receiver.as_call_node() {
            if let Some(dot_loc) = call.call_operator_loc() {
                if dot_loc.start_offset() == fc_offset {
                    // first_call IS the direct receiver — RuboCop returns false
                    return false;
                }
            }
        }
        // first_call differs from direct receiver — check dot position
        return fc_line == line && fc_col == target_col;
    }
    false
}

/// Walk down the receiver chain to find the root (node with no receiver),
/// then return the first call with a dot above it (the deepest call in
/// the chain that has a dot operator). Returns (line, col, byte_offset).
fn find_first_call_dot(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Option<(usize, usize, usize)> {
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            // Try deeper first
            if let Some(deeper) = find_first_call_dot(source, &recv) {
                return Some(deeper);
            }
        }
        // This is the deepest call with a dot (or the only call)
        if let Some(dot_loc) = call.call_operator_loc() {
            let (dot_line, dot_col) = source.offset_to_line_col(dot_loc.start_offset());
            return Some((dot_line, dot_col, dot_loc.start_offset()));
        }
    }
    None
}

/// Check if the chain root is inside a keyword expression (return, if, while,
/// until, for) and return the extra indentation width if so. RuboCop adds an
/// extra IndentationWidth for chains inside these keyword expressions when
/// using `indented` style.
fn keyword_extra_indent(
    source: &SourceFile,
    call_node: &ruby_prism::CallNode<'_>,
    _width: usize,
) -> usize {
    // Check the chain start line for a keyword prefix
    let receiver = match call_node.receiver() {
        Some(r) => r,
        None => return 0,
    };
    let chain_start_line = find_chain_start_line(source, &receiver);
    let chain_line_bytes = source
        .lines()
        .nth(chain_start_line - 1)
        .unwrap_or(b"");
    // Get the text after indentation
    let trimmed = chain_line_bytes.iter().skip_while(|&&b| b == b' ' || b == b'\t');
    let text: Vec<u8> = trimmed.copied().collect();
    // Check for keyword prefixes: return, if, while, until, for, unless
    let keywords: &[&[u8]] = &[b"return ", b"return(", b"if ", b"while ", b"until ", b"for ", b"unless "];
    for kw in keywords {
        if text.starts_with(kw) {
            // RuboCop adds IndentationWidth for the keyword, but the configured
            // cop IndentationWidth is what we use. In practice, both are 2.
            return 2;
        }
    }
    0
}

/// Find the start column of the chain root (deepest receiver).
fn find_chain_root_col(source: &SourceFile, node: &ruby_prism::Node<'_>) -> usize {
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            return find_chain_root_col(source, &recv);
        }
    }
    // Also walk through blocks (for block chain patterns)
    if let Some(block) = node.as_block_node() {
        // A block's "receiver" in Prism terms is implicit; check the block's
        // start position which corresponds to the call that owns it
        let (_, col) = source.offset_to_line_col(block.location().start_offset());
        return col;
    }
    let (_, col) = source.offset_to_line_col(node.location().start_offset());
    col
}

/// Walk backwards from a given line to find the first line that does NOT
/// start with a continuation dot (`.` or `&.` at the start of the trimmed
/// line). This handles sub-chains that start on continuation dot lines,
/// matching RuboCop's `left_hand_side` which walks up through parent nodes
/// to find the topmost chain start.
fn find_non_continuation_ancestor_line(source: &SourceFile, start_line: usize) -> usize {
    let lines: Vec<&[u8]> = source.lines().collect();
    let mut line = start_line;
    while line >= 1 {
        if line - 1 >= lines.len() {
            break;
        }
        let line_bytes = lines[line - 1];
        let trimmed: Vec<u8> = line_bytes
            .iter()
            .copied()
            .skip_while(|&b| b == b' ' || b == b'\t')
            .collect();
        if trimmed.starts_with(b".") || trimmed.starts_with(b"&.") {
            // This line is a continuation dot line; walk backwards
            if line <= 1 {
                break;
            }
            line -= 1;
        } else {
            break;
        }
    }
    line
}

fn find_chain_start_line(source: &SourceFile, node: &ruby_prism::Node<'_>) -> usize {
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            let (recv_line, _) = source.offset_to_line_col(recv.location().start_offset());
            let (call_msg_line, _) = if let Some(dot_loc) = call.call_operator_loc() {
                source.offset_to_line_col(dot_loc.start_offset())
            } else {
                (recv_line, 0)
            };
            // If this call is also multiline chained, recurse
            if call_msg_line != recv_line {
                return find_chain_start_line(source, &recv);
            }
        }
    }
    let (line, _) = source.offset_to_line_col(node.location().start_offset());
    line
}

/// Find a description of the alignment base for error messages.
/// Returns (base_source_name, base_line).
/// Follows the same logic as `find_alignment_base_col` to determine which
/// node is the actual alignment target.
fn find_alignment_base_description(
    source: &SourceFile,
    receiver: &ruby_prism::Node<'_>,
) -> (String, usize) {
    // Check for block chain continuation first — when the receiver is a
    // call-with-block whose dot+method+block is single-line, the alignment
    // base is that call's dot+selector.
    if let Some(call) = receiver.as_call_node() {
        if call.block().is_some() {
            if let Some(dot_loc) = call.call_operator_loc() {
                let (dot_line, _) = source.offset_to_line_col(dot_loc.start_offset());
                let loc = call.location();
                let (end_line, _) = source.offset_to_line_col(loc.end_offset());
                if dot_line == end_line {
                    let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("?");
                    return (format!(".{name}"), dot_line);
                }
            }
        }
    }

    // Walk down to the chain root
    let (root, root_line) = find_chain_root_info(source, receiver);
    (root, root_line)
}

/// Walk down the receiver chain to find the root and its description.
fn find_chain_root_info(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> (String, usize) {
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            return find_chain_root_info(source, &recv);
        }
        // No receiver — this call IS the root (e.g., `be_an(Array)`)
        let name = call.name().as_slice();
        let name_str = std::str::from_utf8(name).unwrap_or("?");
        // Include arguments in the display for cases like `be_an(Array)`
        let loc = call.location();
        let (line, _) = source.offset_to_line_col(loc.start_offset());
        let source_text = extract_call_source(source, call);
        return (source_text.unwrap_or_else(|| name_str.to_string()), line);
    }
    if let Some(block) = node.as_block_node() {
        let (_, col) = source.offset_to_line_col(block.location().start_offset());
        let (line, _) = source.offset_to_line_col(block.location().start_offset());
        let _ = col;
        return ("...".to_string(), line);
    }
    // For local variables, instance variables, constants, etc.
    let loc = node.location();
    let (line, _) = source.offset_to_line_col(loc.start_offset());
    let name = std::str::from_utf8(loc.as_slice()).unwrap_or("?");
    // Trim to just the identifier (no trailing whitespace/newlines)
    let name = name.split_whitespace().next().unwrap_or("?");
    (name.to_string(), line)
}

/// Extract a concise source representation of a call for messages.
fn extract_call_source(
    _source: &SourceFile,
    call: ruby_prism::CallNode<'_>,
) -> Option<String> {
    let name = std::str::from_utf8(call.name().as_slice()).ok()?;
    if let Some(args) = call.arguments() {
        let first_arg = args.arguments().iter().next()?;
        let arg_loc = first_arg.location();
        let arg_text = std::str::from_utf8(arg_loc.as_slice()).ok()?;
        Some(format!("{name}({arg_text})"))
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        MultilineMethodCallIndentation,
        "cops/layout/multiline_method_call_indentation"
    );

    #[test]
    fn same_line_chain_ignored() {
        let source = b"foo.bar.baz\n";
        let diags = run_cop_full(&MultilineMethodCallIndentation, source);
        assert!(diags.is_empty());
    }
}
