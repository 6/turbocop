use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct OutOfRangeRegexpRef;

impl Cop for OutOfRangeRegexpRef {
    fn name(&self) -> &'static str {
        "Lint/OutOfRangeRegexpRef"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = RegexpRefVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            current_capture_count: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RegexpRefVisitor<'a, 'src> {
    cop: &'a OutOfRangeRegexpRef,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Number of capture groups in the most recent regexp match.
    /// None means no regexp match has been seen yet.
    current_capture_count: Option<usize>,
}

impl<'pr> Visit<'pr> for RegexpRefVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method = node.name().as_slice();

        // `=~` operator
        if method == b"=~" {
            let mut found_captures = false;
            if let Some(args) = node.arguments() {
                let arg_list: Vec<ruby_prism::Node<'pr>> = args.arguments().iter().collect();
                if let Some(arg) = arg_list.first() {
                    // RHS regexp takes precedence
                    if let Some(count) = count_captures_in_node(arg) {
                        self.current_capture_count = Some(count);
                        found_captures = true;
                    } else if let Some(recv) = node.receiver() {
                        // LHS regexp
                        if let Some(count) = count_captures_in_node(&recv) {
                            self.current_capture_count = Some(count);
                            found_captures = true;
                        }
                    }
                }
            }
            // If neither side is a recognizable literal regexp (e.g., one is a constant
            // reference), mark capture count as unknown so we don't false-positive on $N.
            if !found_captures {
                self.current_capture_count = Some(usize::MAX);
            }
            ruby_prism::visit_call_node(self, node);
            return;
        }

        // `===` operator with regexp receiver
        if method == b"===" {
            let mut found = false;
            if let Some(recv) = node.receiver() {
                if let Some(count) = count_captures_in_node(&recv) {
                    self.current_capture_count = Some(count);
                    found = true;
                }
            }
            if !found {
                // Receiver may be a constant regexp reference — mark as unknown
                self.current_capture_count = Some(usize::MAX);
            }
            ruby_prism::visit_call_node(self, node);
            return;
        }

        // `match` method with regexp receiver (but not `match?`)
        if method == b"match" {
            let mut found = false;
            if let Some(recv) = node.receiver() {
                if let Some(count) = count_captures_in_node(&recv) {
                    if node.arguments().is_some() {
                        self.current_capture_count = Some(count);
                        found = true;
                    }
                } else if let Some(args) = node.arguments() {
                    // match with regexp as argument
                    let arg_list: Vec<ruby_prism::Node<'pr>> = args.arguments().iter().collect();
                    if let Some(arg) = arg_list.first() {
                        if let Some(count) = count_captures_in_node(arg) {
                            self.current_capture_count = Some(count);
                            found = true;
                        }
                    }
                }
            }
            if !found && node.arguments().is_some() {
                // Regexp is a constant or dynamic reference — mark as unknown
                self.current_capture_count = Some(usize::MAX);
            }
            ruby_prism::visit_call_node(self, node);
            return;
        }

        // `match?` does NOT update $1, $2, etc.
        if method == b"match?" {
            ruby_prism::visit_call_node(self, node);
            return;
        }

        // Methods that take a regexp arg and set backreferences:
        // gsub, gsub!, sub, sub!, scan, slice, slice!, index, rindex,
        // partition, rpartition, start_with?, end_with?, []
        let sets_backref = matches!(
            method,
            b"gsub" | b"gsub!" | b"sub" | b"sub!" | b"scan"
                | b"slice" | b"slice!" | b"index" | b"rindex"
                | b"partition" | b"rpartition" | b"start_with?" | b"end_with?"
                | b"[]" | b"grep"
        );

        if sets_backref {
            if let Some(args) = node.arguments() {
                let arg_list: Vec<ruby_prism::Node<'pr>> = args.arguments().iter().collect();
                if let Some(arg) = arg_list.first() {
                    if let Some(count) = count_captures_in_node(arg) {
                        self.current_capture_count = Some(count);
                    } else {
                        // Non-literal regexp argument (variable, constant, etc.) —
                        // captures can't be determined statically, mark as unknown
                        self.current_capture_count = Some(usize::MAX);
                    }
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }

    fn visit_match_write_node(&mut self, node: &ruby_prism::MatchWriteNode<'pr>) {
        // This is for `/regexp/ =~ string` where the regexp is on the LHS
        let call = node.call();
        if let Some(recv) = call.receiver() {
            if let Some(count) = count_captures_in_node(&recv) {
                self.current_capture_count = Some(count);
            }
        }
        ruby_prism::visit_match_write_node(self, node);
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        // For case/when, each when clause with regexp conditions sets capture count
        let saved = self.current_capture_count;
        for condition in node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                let mut max_captures = 0;
                for cond in when_node.conditions().iter() {
                    if let Some(count) = count_captures_in_node(&cond) {
                        max_captures = max_captures.max(count);
                    }
                }
                if max_captures > 0 || when_node.conditions().iter().any(|c| count_captures_in_node(&c).is_some()) {
                    self.current_capture_count = Some(max_captures);
                }
                if let Some(body) = when_node.statements() {
                    self.visit_statements_node(&body);
                }
            }
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
        self.current_capture_count = saved;
    }

    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        let saved = self.current_capture_count;
        for condition in node.conditions().iter() {
            if let Some(in_node) = condition.as_in_node() {
                let max_captures = count_captures_in_pattern(&in_node.pattern());
                if max_captures > 0 {
                    self.current_capture_count = Some(max_captures);
                }
                if let Some(body) = in_node.statements() {
                    self.visit_statements_node(&body);
                }
            }
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
        self.current_capture_count = saved;
    }

    fn visit_numbered_reference_read_node(
        &mut self,
        node: &ruby_prism::NumberedReferenceReadNode<'pr>,
    ) {
        let ref_num = node.number() as usize;
        if let Some(max_captures) = self.current_capture_count {
            if ref_num > max_captures {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                let message = if max_captures == 0 {
                    format!("${} is out of range (no regexp capture groups detected).", ref_num)
                } else if max_captures == 1 {
                    format!("${} is out of range ({} regexp capture group detected).", ref_num, max_captures)
                } else {
                    format!("${} is out of range ({} regexp capture groups detected).", ref_num, max_captures)
                };
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    message,
                ));
            }
        } else {
            // No regexp match seen — any $N reference is out of range
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("${} is out of range (no regexp capture groups detected).", ref_num),
            ));
        }
    }
}

/// Count capture groups in a regexp node. Returns None if not a literal regexp.
fn count_captures_in_node(node: &ruby_prism::Node<'_>) -> Option<usize> {
    if let Some(regexp) = node.as_regular_expression_node() {
        // Check for interpolation — skip if present
        let content = regexp.unescaped();
        let content_str = std::str::from_utf8(&content).ok()?;
        Some(count_capture_groups(content_str))
    } else if let Some(interp_regexp) = node.as_interpolated_regular_expression_node() {
        // If it has interpolation, we can't reliably count captures
        let mut has_interp = false;
        let mut pattern = String::new();
        for part in interp_regexp.parts().iter() {
            if let Some(s) = part.as_string_node() {
                let val = s.unescaped();
                pattern.push_str(&String::from_utf8_lossy(&val));
            } else {
                has_interp = true;
            }
        }
        if has_interp {
            return None; // Can't count with interpolation
        }
        Some(count_capture_groups(&pattern))
    } else {
        None
    }
}

/// Count capture groups in a regexp pattern string.
fn count_capture_groups(pattern: &str) -> usize {
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut count = 0;
    let mut named_count = 0;

    while i < len {
        if bytes[i] == b'\\' {
            i += 2; // skip escaped char
            continue;
        }

        // Skip character classes
        if bytes[i] == b'[' {
            i += 1;
            if i < len && bytes[i] == b'^' {
                i += 1;
            }
            if i < len && bytes[i] == b']' {
                i += 1;
            }
            while i < len && bytes[i] != b']' {
                if bytes[i] == b'\\' {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        if bytes[i] == b'(' && i + 1 < len {
            if bytes[i + 1] == b'?' {
                if i + 2 < len {
                    match bytes[i + 2] {
                        b'<' => {
                            if i + 3 < len && bytes[i + 3] != b'=' && bytes[i + 3] != b'!' {
                                named_count += 1;
                            }
                        }
                        b'\'' => {
                            named_count += 1;
                        }
                        _ => {} // non-capturing
                    }
                }
            } else {
                count += 1;
            }
        }

        i += 1;
    }

    // If there are named captures, only named captures count for $N references
    // Named captures disable numbered captures in Ruby
    if named_count > 0 {
        named_count
    } else {
        count
    }
}

/// Count max captures in a pattern matching expression.
fn count_captures_in_pattern(node: &ruby_prism::Node<'_>) -> usize {
    let mut max = 0;

    if let Some(count) = count_captures_in_node(node) {
        max = max.max(count);
    }

    // Check array patterns
    if let Some(arr) = node.as_array_pattern_node() {
        for elem in arr.requireds().iter() {
            max = max.max(count_captures_in_pattern(&elem));
        }
    }

    // Check hash patterns
    if let Some(hash) = node.as_hash_pattern_node() {
        for elem in hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                max = max.max(count_captures_in_pattern(&assoc.value()));
            }
        }
    }

    // Check alternation patterns
    if let Some(alt) = node.as_alternation_pattern_node() {
        max = max.max(count_captures_in_pattern(&alt.left()));
        max = max.max(count_captures_in_pattern(&alt.right()));
    }

    // Check capture patterns (=> var)
    if let Some(cap) = node.as_capture_pattern_node() {
        max = max.max(count_captures_in_pattern(&cap.value()));
    }

    // Check pinned patterns
    if let Some(pin) = node.as_pinned_variable_node() {
        max = max.max(count_captures_in_pattern(&pin.variable()));
    }

    max
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OutOfRangeRegexpRef, "cops/lint/out_of_range_regexp_ref");
}
