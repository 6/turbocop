use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

/// Count body lines between start and end offsets (exclusive of keyword lines).
/// Skips blank lines. Optionally skips comment-only lines.
pub fn count_body_lines(
    source: &SourceFile,
    start_offset: usize,
    end_offset: usize,
    count_comments: bool,
) -> usize {
    count_body_lines_ex(source, start_offset, end_offset, count_comments, &[])
}

/// Count body lines with foldable line ranges.
/// `foldable_ranges` contains (start_line, end_line) pairs (1-indexed) of multiline
/// constructs that should count as a single line instead of their actual line count.
pub fn count_body_lines_ex(
    source: &SourceFile,
    start_offset: usize,
    end_offset: usize,
    count_comments: bool,
    foldable_ranges: &[(usize, usize)],
) -> usize {
    let (start_line, _) = source.offset_to_line_col(start_offset);
    let (end_line, _) = source.offset_to_line_col(end_offset);

    // Build a set of lines that are "folded away" (continuation lines of foldable constructs)
    let mut folded_lines = std::collections::HashSet::new();
    for &(fold_start, fold_end) in foldable_ranges {
        // The first line counts as 1, additional lines are folded
        for line in (fold_start + 1)..=fold_end {
            folded_lines.insert(line);
        }
    }

    // Count lines between (exclusive of def/end lines)
    let lines: Vec<&[u8]> = source.lines().collect();
    let mut count = 0;

    // Lines between start_line and end_line (exclusive)
    // start_line and end_line are 1-indexed
    for line_num in (start_line + 1)..end_line {
        if line_num > lines.len() {
            break;
        }

        // Skip folded continuation lines
        if folded_lines.contains(&line_num) {
            continue;
        }

        let line = lines[line_num - 1]; // convert to 0-indexed
        let trimmed = trim_bytes(line);

        // Skip blank lines
        if trimmed.is_empty() {
            continue;
        }

        // Optionally skip comment-only lines
        if !count_comments && trimmed.starts_with(b"#") {
            continue;
        }

        count += 1;
    }

    count
}

/// Collect line ranges of foldable constructs within a node.
/// `count_as_one` contains type names like "array", "hash", "heredoc", "method_call".
/// Returns pairs of (start_line, end_line) (1-indexed) for multiline foldable constructs.
pub fn collect_foldable_ranges(
    source: &SourceFile,
    body_node: &ruby_prism::Node<'_>,
    count_as_one: &[String],
) -> Vec<(usize, usize)> {
    use ruby_prism::Visit;
    let mut visitor = FoldableVisitor {
        source,
        count_as_one,
        ranges: Vec::new(),
    };
    visitor.visit(body_node);
    visitor.ranges
}

struct FoldableVisitor<'a> {
    source: &'a SourceFile,
    count_as_one: &'a [String],
    ranges: Vec<(usize, usize)>,
}

impl<'pr> ruby_prism::Visit<'pr> for FoldableVisitor<'_> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        let is_foldable = match &node {
            ruby_prism::Node::ArrayNode { .. } => self.count_as_one.iter().any(|s| s == "array"),
            ruby_prism::Node::HashNode { .. } => self.count_as_one.iter().any(|s| s == "hash"),
            ruby_prism::Node::InterpolatedStringNode { .. } => {
                self.count_as_one.iter().any(|s| s == "heredoc")
            }
            ruby_prism::Node::CallNode { .. } => {
                self.count_as_one.iter().any(|s| s == "method_call")
            }
            _ => false,
        };

        if is_foldable {
            let loc = node.location();
            let (start_line, _) = self.source.offset_to_line_col(loc.start_offset());
            let end_off = loc.end_offset().saturating_sub(1).max(loc.start_offset());
            let (end_line, _) = self.source.offset_to_line_col(end_off);
            if end_line > start_line {
                self.ranges.push((start_line, end_line));
                return; // Don't recurse into foldable construct
            }
        }
    }
}

fn trim_bytes(b: &[u8]) -> &[u8] {
    let start = b.iter().position(|&c| c != b' ' && c != b'\t' && c != b'\r');
    match start {
        Some(s) => {
            let end = b.iter().rposition(|&c| c != b' ' && c != b'\t' && c != b'\r').unwrap();
            &b[s..=end]
        }
        None => &[],
    }
}

/// Check if a name is snake_case (lowercase + digits + underscores, not starting with uppercase).
pub fn is_snake_case(name: &[u8]) -> bool {
    if name.is_empty() {
        return true;
    }
    // Allow leading underscores (e.g., _foo)
    // Must not contain uppercase letters
    for &b in name {
        if b.is_ascii_uppercase() {
            return false;
        }
        if !(b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
            // Allow ? and ! at end for Ruby method names
            if b == b'?' || b == b'!' || b == b'=' {
                continue;
            }
            return false;
        }
    }
    true
}

/// Check if a name is SCREAMING_SNAKE_CASE (uppercase + digits + underscores).
pub fn is_screaming_snake_case(name: &[u8]) -> bool {
    if name.is_empty() {
        return true;
    }
    for &b in name {
        if b.is_ascii_lowercase() {
            return false;
        }
        if !(b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') {
            return false;
        }
    }
    true
}

/// Check if a name is CamelCase (starts uppercase, no underscores).
pub fn is_camel_case(name: &[u8]) -> bool {
    if name.is_empty() {
        return false;
    }
    if !name[0].is_ascii_uppercase() {
        return false;
    }
    // Allow digits, no underscores (except leading _ is not CamelCase)
    for &b in &name[1..] {
        if b == b'_' {
            return false;
        }
        if !(b.is_ascii_alphanumeric()) {
            return false;
        }
    }
    true
}

/// Check if all bytes in a name are ASCII.
pub fn is_ascii_name(name: &[u8]) -> bool {
    name.iter().all(|b| b.is_ascii())
}

/// Info about a 2-method chain: `receiver.inner_method(...).outer_method(...)`.
pub struct MethodChain<'a> {
    /// The inner CallNode (the receiver of the outer call).
    pub inner_call: ruby_prism::CallNode<'a>,
    /// The method name of the inner call.
    pub inner_method: &'a [u8],
    /// The method name of the outer call.
    pub outer_method: &'a [u8],
}

/// Extract a 2-method chain from a node.
///
/// If `node` is a CallNode `x.outer()` whose receiver is also a CallNode `y.inner()`,
/// returns `Some(MethodChain { inner_call, inner_method, outer_method })`.
pub fn as_method_chain<'a>(node: &ruby_prism::Node<'a>) -> Option<MethodChain<'a>> {
    let outer_call = node.as_call_node()?;
    let outer_method = outer_call.name().as_slice();
    let receiver = outer_call.receiver()?;
    let inner_call = receiver.as_call_node()?;
    let inner_method = inner_call.name().as_slice();
    Some(MethodChain {
        inner_call,
        inner_method,
        outer_method,
    })
}

/// Check if the line above a node's start offset is a comment line.
pub fn preceding_comment_line(source: &SourceFile, node_start_offset: usize) -> bool {
    let (node_line, _) = source.offset_to_line_col(node_start_offset);
    if node_line <= 1 {
        return false;
    }
    let lines: Vec<&[u8]> = source.lines().collect();
    let prev_line = lines.get(node_line - 2); // node_line is 1-indexed, prev is node_line-1, 0-indexed = node_line-2
    match prev_line {
        Some(line) => {
            let trimmed = trim_bytes(line);
            trimmed.starts_with(b"#")
        }
        None => false,
    }
}

/// Check if a node spans exactly one line in the source.
pub fn node_on_single_line(source: &SourceFile, loc: &ruby_prism::Location<'_>) -> bool {
    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
    let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(end_offset);
    start_line == end_line
}

/// Compute the expected indentation column for body statements
/// given the keyword's column and the configured width.
pub fn expected_indent_for_body(keyword_column: usize, width: usize) -> usize {
    keyword_column + width
}

/// Get the line content at a given 1-indexed line number.
pub fn line_at(source: &SourceFile, line_number: usize) -> Option<&[u8]> {
    source.lines().nth(line_number - 1)
}

/// Get the indentation (number of leading spaces) for a byte slice.
pub fn indentation_of(line: &[u8]) -> usize {
    line.iter().take_while(|&&b| b == b' ').count()
}

/// Check if there is a trailing comma between last_element_end and closing_start.
pub fn has_trailing_comma(
    source_bytes: &[u8],
    last_element_end: usize,
    closing_start: usize,
) -> bool {
    if last_element_end >= closing_start || closing_start > source_bytes.len() {
        return false;
    }
    source_bytes[last_element_end..closing_start]
        .iter()
        .any(|&b| b == b',')
}

// ── Shared cop logic helpers ──────────────────────────────────────────

/// Check if a line is blank (only whitespace).
pub fn is_blank_line(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

/// Check for extra empty lines at the beginning/end of a body.
/// Used by EmptyLinesAround{Class,Module,Method,Block}Body.
pub fn check_empty_lines_around_body(
    cop_name: &str,
    source: &SourceFile,
    keyword_offset: usize,
    end_offset: usize,
    body_kind: &str,
) -> Vec<Diagnostic> {
    let (keyword_line, _) = source.offset_to_line_col(keyword_offset);
    let (end_line, _) = source.offset_to_line_col(end_offset);

    if keyword_line == end_line {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();

    // Check for blank line after keyword
    let after_keyword = keyword_line + 1;
    if let Some(line) = line_at(source, after_keyword) {
        if is_blank_line(line) && after_keyword < end_line {
            diagnostics.push(Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line: after_keyword, column: 0 },
                severity: Severity::Convention,
                cop_name: cop_name.to_string(),
                message: format!("Extra empty line detected at {body_kind} body beginning."),
            });
        }
    }

    // Check for blank line before end
    if end_line > 1 {
        let before_end = end_line - 1;
        if before_end > keyword_line {
            if let Some(line) = line_at(source, before_end) {
                if is_blank_line(line) {
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line: before_end, column: 0 },
                        severity: Severity::Convention,
                        cop_name: cop_name.to_string(),
                        message: format!("Extra empty line detected at {body_kind} body end."),
                    });
                }
            }
        }
    }

    diagnostics
}

/// Check for MISSING empty lines at the beginning/end of a body.
/// Used by EmptyLinesAround{Block,Class,Module}Body with "empty_lines" style.
pub fn check_missing_empty_lines_around_body(
    cop_name: &str,
    source: &SourceFile,
    keyword_offset: usize,
    end_offset: usize,
    body_kind: &str,
) -> Vec<Diagnostic> {
    let (keyword_line, _) = source.offset_to_line_col(keyword_offset);
    let (end_line, _) = source.offset_to_line_col(end_offset);

    // Skip single-line or empty bodies
    if end_line <= keyword_line + 1 {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();

    // Check for missing blank line after keyword
    let after_keyword = keyword_line + 1;
    if let Some(line) = line_at(source, after_keyword) {
        if !is_blank_line(line) && after_keyword < end_line {
            diagnostics.push(Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line: after_keyword, column: 0 },
                severity: Severity::Convention,
                cop_name: cop_name.to_string(),
                message: format!("Empty line missing at {body_kind} body beginning."),
            });
        }
    }

    // Check for missing blank line before end
    if end_line > 1 {
        let before_end = end_line - 1;
        if before_end > keyword_line {
            if let Some(line) = line_at(source, before_end) {
                if !is_blank_line(line) {
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line: before_end, column: 0 },
                        severity: Severity::Convention,
                        cop_name: cop_name.to_string(),
                        message: format!("Empty line missing at {body_kind} body end."),
                    });
                }
            }
        }
    }

    diagnostics
}

/// Check that `end` is aligned with the opening keyword.
/// Used by DefEndAlignment, EndAlignment, ElseAlignment.
pub fn check_keyword_end_alignment(
    cop_name: &str,
    source: &SourceFile,
    keyword_name: &str,
    keyword_offset: usize,
    end_offset: usize,
) -> Vec<Diagnostic> {
    // Use the indentation of the line containing the keyword (not the keyword column),
    // because modifiers like `private_class_method def ...` put `def` further right.
    let line_indent = {
        let bytes = source.as_bytes();
        let mut line_start = keyword_offset;
        while line_start > 0 && bytes[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        let mut indent = 0;
        while line_start + indent < bytes.len() && bytes[line_start + indent] == b' ' {
            indent += 1;
        }
        indent
    };
    let (end_line, end_col) = source.offset_to_line_col(end_offset);

    if end_col != line_indent {
        return vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line: end_line, column: end_col },
            severity: Severity::Convention,
            cop_name: cop_name.to_string(),
            message: format!("Align `end` with `{keyword_name}`."),
        }];
    }

    Vec::new()
}

/// Check if the given byte offset is the first non-whitespace character on its line.
/// Matches RuboCop's `begins_its_line?` helper.
pub fn begins_its_line(source: &SourceFile, offset: usize) -> bool {
    let bytes = source.as_bytes();
    let mut pos = offset;
    while pos > 0 && bytes[pos - 1] != b'\n' {
        pos -= 1;
    }
    while pos < offset {
        if bytes[pos] != b' ' && bytes[pos] != b'\t' {
            return false;
        }
        pos += 1;
    }
    true
}

/// Check first element indentation relative to an opening delimiter.
/// Used by FirstArgument/Array/HashElementIndentation.
pub fn check_first_element_indentation(
    cop_name: &str,
    source: &SourceFile,
    width: usize,
    opening_offset: usize,
    first_element_offset: usize,
) -> Vec<Diagnostic> {
    let (open_line, _) = source.offset_to_line_col(opening_offset);
    let (elem_line, elem_col) = source.offset_to_line_col(first_element_offset);

    // Skip if on same line as opener
    if elem_line == open_line {
        return Vec::new();
    }

    let open_line_bytes = source.lines().nth(open_line - 1).unwrap_or(b"");
    let open_indent = indentation_of(open_line_bytes);
    let expected = open_indent + width;

    if elem_col != expected {
        return vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line: elem_line, column: elem_col },
            severity: Severity::Convention,
            cop_name: cop_name.to_string(),
            message: format!(
                "Use {} (not {}) spaces for indentation of the first element.",
                width,
                elem_col.saturating_sub(open_indent)
            ),
        }];
    }

    Vec::new()
}

// ── Rails-specific helpers ─────────────────────────────────────────────

/// Extract the superclass constant name from a ClassNode.
///
/// For `class Foo < ActiveRecord::Base`, returns `Some(b"ActiveRecord::Base")`.
/// Returns `None` if the class has no superclass or the superclass isn't a
/// simple constant or constant path.
pub fn parent_class_name<'a>(
    source: &'a SourceFile,
    class_node: &ruby_prism::ClassNode<'a>,
) -> Option<&'a [u8]> {
    let superclass = class_node.superclass()?;
    let loc = superclass.location();
    Some(&source.as_bytes()[loc.start_offset()..loc.end_offset()])
}

/// Check if a CallNode is a receiverless DSL-style call with the given method name.
///
/// Matches patterns like `has_many`, `validates`, `before_action` etc.
pub fn is_dsl_call(call: &ruby_prism::CallNode<'_>, name: &[u8]) -> bool {
    call.receiver().is_none() && call.name().as_slice() == name
}

/// Get all direct call statements from a class body's StatementsNode.
///
/// Returns an iterator over CallNode entries in the class body at the top level
/// (not nested inside methods).
pub fn class_body_calls<'a>(
    class_node: &ruby_prism::ClassNode<'a>,
) -> Vec<ruby_prism::CallNode<'a>> {
    let body = match class_node.body() {
        Some(b) => b,
        None => return Vec::new(),
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return Vec::new(),
    };
    stmts
        .body()
        .iter()
        .filter_map(|node| node.as_call_node())
        .collect()
}

/// Check if a CallNode has a keyword argument with the given key name.
///
/// Looks for `key: value` in the call's argument list.
pub fn has_keyword_arg(call: &ruby_prism::CallNode<'_>, key: &[u8]) -> bool {
    keyword_arg_value(call, key).is_some()
}

/// Get the value node of a keyword argument with the given key name.
///
/// For `has_many :items, dependent: :destroy`, `keyword_arg_value(call, b"dependent")`
/// returns the SymbolNode for `:destroy`.
pub fn keyword_arg_value<'a>(
    call: &ruby_prism::CallNode<'a>,
    key: &[u8],
) -> Option<ruby_prism::Node<'a>> {
    let args = call.arguments()?;
    for arg in args.arguments().iter() {
        // Direct keyword hash pairs in arguments
        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if sym.unescaped() == key {
                            return Some(assoc.value());
                        }
                    }
                }
            }
        }
        // Hash literal as last argument
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if sym.unescaped() == key {
                            return Some(assoc.value());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Get the constant name (last segment) from a constant path or constant read node.
///
/// For `ActiveRecord::Base`, returns `b"Base"`.
/// For `User`, returns `b"User"`.
pub fn constant_name<'a>(node: &ruby_prism::Node<'a>) -> Option<&'a [u8]> {
    if let Some(cr) = node.as_constant_read_node() {
        return Some(cr.name().as_slice());
    }
    if let Some(cp) = node.as_constant_path_node() {
        if let Some(name_node) = cp.name() {
            return Some(name_node.as_slice());
        }
    }
    None
}

/// Get the full constant path string from source bytes.
///
/// For a ConstantPathNode like `ActiveRecord::Base`, extracts the full text.
pub fn full_constant_path<'a>(
    source: &'a SourceFile,
    node: &ruby_prism::Node<'_>,
) -> &'a [u8] {
    let loc = node.location();
    &source.as_bytes()[loc.start_offset()..loc.end_offset()]
}

/// Extract a 3-method chain from a node.
///
/// If `node` is a CallNode `x.c()` whose receiver is `y.b()` whose receiver is `z.a()`,
/// returns the three method names and call nodes.
pub struct MethodChain3<'a> {
    pub innermost_call: ruby_prism::CallNode<'a>,
    pub innermost_method: &'a [u8],
    pub middle_method: &'a [u8],
    pub outer_method: &'a [u8],
}

pub fn as_method_chain3<'a>(node: &ruby_prism::Node<'a>) -> Option<MethodChain3<'a>> {
    let outer_call = node.as_call_node()?;
    let outer_method = outer_call.name().as_slice();
    let mid_recv = outer_call.receiver()?;
    let mid_call = mid_recv.as_call_node()?;
    let middle_method = mid_call.name().as_slice();
    let inner_recv = mid_call.receiver()?;
    let innermost_call = inner_recv.as_call_node()?;
    let innermost_method = innermost_call.name().as_slice();
    Some(MethodChain3 {
        innermost_call,
        innermost_method,
        middle_method,
        outer_method,
    })
}

// ── RSpec-specific helpers ──────────────────────────────────────────────

/// RSpec example group methods.
pub const RSPEC_EXAMPLE_GROUPS: &[&str] = &[
    "describe", "context", "feature", "example_group",
    "xdescribe", "xcontext", "xfeature",
    "fdescribe", "fcontext", "ffeature",
    "shared_examples", "shared_examples_for", "shared_context",
];

/// RSpec focused (f-prefixed) methods.
pub const RSPEC_FOCUSED_METHODS: &[&str] = &[
    "fdescribe", "fcontext", "ffeature",
    "fit", "fspecify", "fexample", "fscenario",
    "focus",
];

/// RSpec example methods.
pub const RSPEC_EXAMPLES: &[&str] = &[
    "it", "specify", "example", "scenario",
    "xit", "xspecify", "xexample", "xscenario",
    "fit", "fspecify", "fexample", "fscenario",
    "pending", "skip",
];

/// RSpec hook methods.
pub const RSPEC_HOOKS: &[&str] = &[
    "before", "after", "around",
    "prepend_before", "prepend_after",
    "append_before", "append_after",
];

/// RSpec let/subject methods.
pub const RSPEC_LETS: &[&str] = &["let", "let!"];
pub const RSPEC_SUBJECTS: &[&str] = &["subject", "subject!"];

/// All RSpec methods that define example groups or examples (for detecting RSpec context).
pub const RSPEC_ALL_METHODS: &[&str] = &[
    "describe", "context", "feature", "example_group",
    "xdescribe", "xcontext", "xfeature",
    "fdescribe", "fcontext", "ffeature",
    "shared_examples", "shared_examples_for", "shared_context",
    "it", "specify", "example", "scenario",
    "xit", "xspecify", "xexample", "xscenario",
    "fit", "fspecify", "fexample", "fscenario",
    "pending", "skip", "focus",
    "before", "after", "around",
    "let", "let!", "subject", "subject!",
];

/// Check if a method name is an RSpec example group method.
pub fn is_rspec_example_group(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    RSPEC_EXAMPLE_GROUPS.contains(&s)
}

/// Check if a method name is an RSpec example method.
pub fn is_rspec_example(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    RSPEC_EXAMPLES.contains(&s)
}

/// Check if a method name is an RSpec hook method.
pub fn is_rspec_hook(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    RSPEC_HOOKS.contains(&s)
}

/// Check if a method name is a focused RSpec method (f-prefixed).
pub fn is_rspec_focused(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    RSPEC_FOCUSED_METHODS.contains(&s)
}

/// Check if a method name is an RSpec let or let!.
pub fn is_rspec_let(name: &[u8]) -> bool {
    name == b"let" || name == b"let!"
}

/// Check if a method name is an RSpec subject or subject!.
pub fn is_rspec_subject(name: &[u8]) -> bool {
    name == b"subject" || name == b"subject!"
}

/// Default include patterns for all RSpec cops — only run on spec files.
pub const RSPEC_DEFAULT_INCLUDE: &[&str] = &["**/*_spec.rb", "**/spec/**/*"];

/// Check if a CallNode has a keyword argument `focus: true` or symbol arg `:focus`.
pub fn has_rspec_focus_metadata(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<(usize, usize, usize, usize)> {
    let call = node.as_call_node()?;
    let args = call.arguments()?;
    for arg in args.arguments().iter() {
        // Check for `:focus` symbol argument
        if let Some(sym) = arg.as_symbol_node() {
            if sym.unescaped() == b"focus" {
                let loc = sym.location();
                let (line, col) = source.offset_to_line_col(loc.start_offset());
                let len = loc.end_offset() - loc.start_offset();
                return Some((line, col, loc.start_offset(), len));
            }
        }
        // Check for `focus: true` keyword argument
        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if sym.unescaped() == b"focus" {
                            let loc = elem.location();
                            let (line, col) = source.offset_to_line_col(loc.start_offset());
                            let len = loc.end_offset() - loc.start_offset();
                            return Some((line, col, loc.start_offset(), len));
                        }
                    }
                }
            }
        }
    }
    None
}

/// Get the first positional (non-keyword) argument from a call node.
pub fn first_positional_arg<'a>(call: &ruby_prism::CallNode<'a>) -> Option<ruby_prism::Node<'a>> {
    let args = call.arguments()?;
    for arg in args.arguments().iter() {
        // Skip keyword hash arguments
        if arg.as_keyword_hash_node().is_some() {
            continue;
        }
        return Some(arg);
    }
    None
}

/// Get the string content of a string node (returns owned Vec).
pub fn string_value(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    if let Some(s) = node.as_string_node() {
        return Some(s.unescaped().to_vec());
    }
    None
}

/// Count block body lines (statements in a block node).
pub fn block_body_line_count(
    source: &SourceFile,
    block: &ruby_prism::BlockNode<'_>,
) -> usize {
    let loc = block.location();
    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
    if end_line <= start_line + 1 { return 0; }
    end_line - start_line - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_snake_case() {
        assert!(is_snake_case(b"foo_bar"));
        assert!(is_snake_case(b"foo"));
        assert!(is_snake_case(b"_foo"));
        assert!(is_snake_case(b"foo_bar_baz"));
        assert!(is_snake_case(b"foo123"));
        assert!(is_snake_case(b"valid?"));
        assert!(is_snake_case(b"save!"));
        assert!(!is_snake_case(b"FooBar"));
        assert!(!is_snake_case(b"fooBar"));
        assert!(!is_snake_case(b"FOO"));
    }

    #[test]
    fn test_is_screaming_snake_case() {
        assert!(is_screaming_snake_case(b"FOO_BAR"));
        assert!(is_screaming_snake_case(b"FOO"));
        assert!(is_screaming_snake_case(b"MAX_SIZE"));
        assert!(is_screaming_snake_case(b"V2"));
        assert!(!is_screaming_snake_case(b"foo_bar"));
        assert!(!is_screaming_snake_case(b"FooBar"));
        assert!(!is_screaming_snake_case(b"Foo"));
    }

    #[test]
    fn test_is_camel_case() {
        assert!(is_camel_case(b"FooBar"));
        assert!(is_camel_case(b"Foo"));
        assert!(is_camel_case(b"FooBarBaz"));
        assert!(is_camel_case(b"Foo123"));
        assert!(!is_camel_case(b"foo_bar"));
        assert!(!is_camel_case(b"FOO_BAR"));
        assert!(!is_camel_case(b"Foo_Bar"));
        assert!(!is_camel_case(b""));
    }

    #[test]
    fn test_is_ascii_name() {
        assert!(is_ascii_name(b"foo_bar"));
        assert!(is_ascii_name(b"FooBar"));
        assert!(!is_ascii_name("café".as_bytes()));
        assert!(!is_ascii_name("naïve".as_bytes()));
    }

    #[test]
    fn test_has_trailing_comma() {
        let src = b"[1, 2, 3,]";
        // '3' ends at byte 8, ']' at byte 9
        assert!(has_trailing_comma(src, 8, 9));
        let src2 = b"[1, 2, 3]";
        // '3' ends at byte 8, ']' at byte 8 — no room for comma
        assert!(!has_trailing_comma(src2, 8, 8));
    }

    #[test]
    fn test_count_body_lines() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"def foo\n  x = 1\n  y = 2\n  # comment\n\n  z = 3\nend\n".to_vec(),
        );
        // def starts at offset 0 (line 1), end starts at offset 45 (line 7)
        // Lines 2-6: "  x = 1", "  y = 2", "  # comment", "", "  z = 3"
        // Without comments: 3 lines (x, y, z)
        assert_eq!(count_body_lines(&source, 0, 45, false), 3);
        // With comments: 4 lines (x, y, #comment, z)
        assert_eq!(count_body_lines(&source, 0, 45, true), 4);
    }
}
