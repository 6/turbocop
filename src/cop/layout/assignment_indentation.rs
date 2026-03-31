/// Checks the indentation of the first line of the right-hand-side of a multi-line assignment.
///
/// ## Investigation findings (2026-03-31)
///
/// **FP root cause (1 FP):** Embedded assignments used as bare method arguments like
/// `body = wrap_file_body path =\n                          File.expand_path(...)`
/// are not chained assignments. The previous source-text scan treated any earlier `=`
/// on the same line as a chain base, so it borrowed `body`'s column instead of `path`'s
/// and falsely flagged the correctly aligned value. Fixed by requiring the current
/// assignment target to begin immediately after only whitespace following the preceding
/// `=`. Direct chains like `foo = bar =` still match, but bare method arguments like
/// `wrap_file_body path =` no longer do.
///
/// ## Investigation findings (2026-03-14)
///
/// **FP root cause (30 FPs):** The cop was using line indentation (`indentation_of`) as the
/// base for expected RHS column, but RuboCop uses the column position of the assignment
/// variable itself. For embedded assignments like `if upload = \n Upload.find_by(...)`,
/// the line indentation includes the `if` keyword, making the base too small and falsely
/// flagging properly-indented RHS. Fixed by using `name_col` (variable column) as the base.
///
/// **FN root cause (94 FNs):** The cop only handled simple write nodes (`*WriteNode`) but
/// missed operator assignments (`+=`, `-=` via `*OperatorWriteNode`), or-assignments
/// (`||=` via `*OrWriteNode`), and-assignments (`&&=` via `*AndWriteNode`),
/// multi-assignments (`a, b = ...` via `MultiWriteNode`), constant path writes
/// (`Module::CONST = ...` via `ConstantPathWriteNode`), setter calls (`obj.x = val`,
/// `hash[key] = val` via `CallNode`), and compound setter/index assignments
/// (`obj.x ||= val`, `hash[key] += val` via `Call*WriteNode`/`Index*WriteNode`). All added.
///
/// ## Investigation findings (2026-03-16, round 2)
///
/// **FP root cause (7 FPs):** The previous chained-assignment fix used line indentation
/// (first non-whitespace column) as the base for all chained assignments. RuboCop's
/// `leftmost_multiple_assignment` goes only ONE level up — it uses the IMMEDIATE PARENT
/// assignment's column, not the outermost. For deeply aligned chains like
/// `@a = @b = @c = \n                @d = nil` (where `@d` is at `@b`'s column + 2),
/// using line indentation produced expected=2 instead of the correct `@b`'s column + 2.
/// Fixed by making `find_chained_assignment_base` return the column of the variable
/// immediately preceding the last `=` sign before the current name, rather than line indent.
///
/// **FN root cause (4 FNs):** Same root cause reversed. For chains at shallow indent like
/// `@a = @b = @c = \n  @d = nil`, the line-indent approach gave expected=2 which matched,
/// but `@b`'s column + 2 is much larger → should be an offense.
///
/// **i18n-tasks FP fix:** Added paren/bracket depth tracking between the preceding `=` and
/// the current name_offset. `keys = ((@cache ||= {})[key] ||= scanner(...))` has unbalanced
/// `(` between `keys =` and the inner `||=`, so it's not a direct chain — returns None.
///
/// **||= / &&= exclusion:** The preceding `=` scanner now rejects `||=` and `&&=` operators
/// (previous char is `|` or `&`), since compound operators are not chained assignments.
///
/// ## Investigation findings (2026-03-16, round 1)
///
/// **FP root cause (25 FPs):** Chained assignments like `a = b = \n value` caused the inner
/// write node to use its own LHS column (`b`'s position) as the indentation base, producing
/// false positives when the value was correctly indented relative to the outermost assignment.
/// RuboCop uses `leftmost_multiple_assignment` to walk up the parent chain to the outermost
/// assignment on the same line. Without parent pointers in Prism, we detect this by scanning
/// the source line before the LHS for a preceding `=` operator. Initial fix used line
/// indentation as base (see round 2 for refinement).
///
/// ## Investigation findings (2026-03-15)
///
/// **FP root cause (149 FPs):** Multiline bracket LHS assignments like
/// `headers[\n  "key"\n] = "value"` were falsely flagged. The cop compared the
/// receiver/name line to the value line to decide if the RHS was on a new line, but
/// RuboCop compares the *operator* (`=`) line to the value line (`same_line?(node.loc.operator, rhs)`).
/// For `] = "value"`, the `=` and value are on the same line, so it's not a multi-line
/// RHS. Fixed by adding an `operator_offset` parameter to `check_write` and using the
/// operator line for the same-line check instead of the name line.
use crate::cop::node_type::{
    CALL_AND_WRITE_NODE, CALL_NODE, CALL_OPERATOR_WRITE_NODE, CALL_OR_WRITE_NODE,
    CLASS_VARIABLE_AND_WRITE_NODE, CLASS_VARIABLE_OPERATOR_WRITE_NODE,
    CLASS_VARIABLE_OR_WRITE_NODE, CLASS_VARIABLE_WRITE_NODE, CONSTANT_AND_WRITE_NODE,
    CONSTANT_OPERATOR_WRITE_NODE, CONSTANT_OR_WRITE_NODE, CONSTANT_PATH_AND_WRITE_NODE,
    CONSTANT_PATH_OPERATOR_WRITE_NODE, CONSTANT_PATH_OR_WRITE_NODE, CONSTANT_PATH_WRITE_NODE,
    CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_AND_WRITE_NODE, GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
    GLOBAL_VARIABLE_OR_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE, INDEX_AND_WRITE_NODE,
    INDEX_OPERATOR_WRITE_NODE, INDEX_OR_WRITE_NODE, INSTANCE_VARIABLE_AND_WRITE_NODE,
    INSTANCE_VARIABLE_OPERATOR_WRITE_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE,
    INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_AND_WRITE_NODE,
    LOCAL_VARIABLE_OPERATOR_WRITE_NODE, LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE,
    MULTI_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AssignmentIndentation;

impl AssignmentIndentation {
    fn is_setter_name(name_bytes: &[u8]) -> bool {
        name_bytes.ends_with(b"=")
            && name_bytes != b"=="
            && name_bytes != b"!="
            && name_bytes != b"==="
            && name_bytes != b"<=>"
            && name_bytes != b">="
            && name_bytes != b"<="
    }

    /// Detect direct chained assignments like `a = b = \n value`.
    ///
    /// The current assignment target must start immediately after only whitespace
    /// following the preceding `=` on the same line. That keeps true chains like
    /// `foo = bar =` while rejecting embedded assignments like
    /// `wrap_file_body path =`, where a method name appears between the two.
    fn find_chained_assignment_base(source: &SourceFile, name_offset: usize) -> Option<usize> {
        let bytes = source.as_bytes();
        let mut line_start = name_offset;
        while line_start > 0 && bytes[line_start - 1] != b'\n' {
            line_start -= 1;
        }

        let prefix = &bytes[line_start..name_offset];
        if prefix.iter().all(|&b| b == b' ' || b == b'\t') {
            return None;
        }

        let mut last_eq_pos: Option<usize> = None;
        let mut i = 0;
        while i < prefix.len() {
            let b = prefix[i];
            if b == b'=' {
                let next = prefix.get(i + 1).copied();
                if next == Some(b'=') {
                    i += if prefix.get(i + 2) == Some(&b'=') {
                        3
                    } else {
                        2
                    };
                    continue;
                }
                let prev = if i > 0 { Some(prefix[i - 1]) } else { None };
                if prev == Some(b'!') || prev == Some(b'>') || prev == Some(b'<') {
                    i += 1;
                    continue;
                }
                if prev == Some(b'|') || prev == Some(b'&') {
                    i += 1;
                    continue;
                }
                last_eq_pos = Some(i);
                i += 1;
                continue;
            }
            if b == b'"' || b == b'\'' {
                let quote = b;
                i += 1;
                while i < prefix.len() && prefix[i] != quote {
                    if prefix[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
                continue;
            }
            i += 1;
        }

        let eq_pos = last_eq_pos?;
        let after_eq = &prefix[eq_pos + 1..];
        if after_eq.iter().any(|&b| b != b' ' && b != b'\t') {
            return None;
        }

        let mut end = eq_pos;
        while end > 0 && (prefix[end - 1] == b' ' || prefix[end - 1] == b'\t') {
            end -= 1;
        }
        if end == 0 {
            return None;
        }

        let token_end = end;
        let mut pos = token_end;
        if pos > 0 && (prefix[pos - 1] == b')' || prefix[pos - 1] == b']') {
            let close = prefix[pos - 1];
            let open = if close == b')' { b'(' } else { b'[' };
            let mut bracket_depth = 1;
            pos -= 1;
            while pos > 0 && bracket_depth > 0 {
                pos -= 1;
                if prefix[pos] == close {
                    bracket_depth += 1;
                } else if prefix[pos] == open {
                    bracket_depth -= 1;
                }
            }
        }
        while pos > 0 {
            let c = prefix[pos - 1];
            if c.is_ascii_alphanumeric()
                || c == b'_'
                || c == b'.'
                || c == b':'
                || c == b'@'
                || c == b'$'
                || c == b'['
                || c == b']'
                || c == b'('
                || c == b')'
            {
                pos -= 1;
            } else {
                break;
            }
        }
        if pos >= token_end {
            return None;
        }

        Some(pos)
    }

    fn check_write(
        &self,
        source: &SourceFile,
        name_offset: usize,
        operator_offset: usize,
        value: &ruby_prism::Node<'_>,
        width: usize,
    ) -> Vec<Diagnostic> {
        let (_name_line, name_col) = source.offset_to_line_col(name_offset);
        let (operator_line, _operator_col) = source.offset_to_line_col(operator_offset);
        let value_loc = value.location();
        let (value_line, value_col) = source.offset_to_line_col(value_loc.start_offset());

        // Only check when RHS is on a different line than the operator (=).
        // For `headers[\n"key"\n] = "value"`, the operator and value are on the
        // same line, so this is not a multi-line RHS — skip it.
        if value_line == operator_line {
            return Vec::new();
        }

        // For chained assignments like `a = b = \n value`, use the column of the
        // outermost (leftmost) assignment on the same line as the base, matching
        // RuboCop's `leftmost_multiple_assignment` behavior. Without parent pointers,
        // we detect this by scanning for a preceding `=` on the same source line.
        let base_col = Self::find_chained_assignment_base(source, name_offset).unwrap_or(name_col);
        let expected = base_col + width;

        if value_col != expected {
            return vec![
                self.diagnostic(
                    source,
                    value_line,
                    value_col,
                    "Indent the first line of the right-hand-side of a multi-line assignment."
                        .to_string(),
                ),
            ];
        }

        Vec::new()
    }
}

impl Cop for AssignmentIndentation {
    fn name(&self) -> &'static str {
        "Layout/AssignmentIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            // Simple writes
            CLASS_VARIABLE_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            // Operator writes (+=, -=, etc.)
            CLASS_VARIABLE_OPERATOR_WRITE_NODE,
            CONSTANT_OPERATOR_WRITE_NODE,
            GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
            INSTANCE_VARIABLE_OPERATOR_WRITE_NODE,
            LOCAL_VARIABLE_OPERATOR_WRITE_NODE,
            // Or writes (||=)
            CLASS_VARIABLE_OR_WRITE_NODE,
            CONSTANT_OR_WRITE_NODE,
            GLOBAL_VARIABLE_OR_WRITE_NODE,
            INSTANCE_VARIABLE_OR_WRITE_NODE,
            LOCAL_VARIABLE_OR_WRITE_NODE,
            // And writes (&&=)
            CLASS_VARIABLE_AND_WRITE_NODE,
            CONSTANT_AND_WRITE_NODE,
            GLOBAL_VARIABLE_AND_WRITE_NODE,
            INSTANCE_VARIABLE_AND_WRITE_NODE,
            LOCAL_VARIABLE_AND_WRITE_NODE,
            // Multi-write (a, b = ...)
            MULTI_WRITE_NODE,
            // Constant path writes (Module::CONST = ...)
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_PATH_AND_WRITE_NODE,
            CONSTANT_PATH_OPERATOR_WRITE_NODE,
            CONSTANT_PATH_OR_WRITE_NODE,
            // Setter calls (obj.x = val, hash[key] = val)
            CALL_NODE,
            CALL_AND_WRITE_NODE,
            CALL_OPERATOR_WRITE_NODE,
            CALL_OR_WRITE_NODE,
            // Index compound writes (hash[key] ||= val, etc.)
            INDEX_AND_WRITE_NODE,
            INDEX_OPERATOR_WRITE_NODE,
            INDEX_OR_WRITE_NODE,
        ]
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
        let width = config.get_usize("IndentationWidth", 2);

        // Simple writes
        if let Some(n) = node.as_local_variable_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_instance_variable_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_class_variable_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_global_variable_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Operator writes (+=, -=, *=, etc.)
        if let Some(n) = node.as_local_variable_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.binary_operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_instance_variable_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.binary_operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_class_variable_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.binary_operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_global_variable_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.binary_operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.binary_operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Or writes (||=)
        if let Some(n) = node.as_local_variable_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_instance_variable_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_class_variable_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_global_variable_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // And writes (&&=)
        if let Some(n) = node.as_local_variable_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_instance_variable_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_class_variable_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_global_variable_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Multi-write (a, b = ...)
        if let Some(n) = node.as_multi_write_node() {
            // Use the start of the whole multi-write node (first target) as the base
            diagnostics.extend(self.check_write(
                source,
                n.location().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Constant path writes (Module::CONST = ...)
        if let Some(n) = node.as_constant_path_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.target().location().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_path_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.target().location().start_offset(),
                n.binary_operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_path_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.target().location().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_path_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.target().location().start_offset(),
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Setter calls (obj.x = val, hash[key] = val)
        if let Some(n) = node.as_call_node() {
            if Self::is_setter_name(n.name().as_slice()) {
                if let Some(args) = n.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if let Some(last_arg) = arg_list.last() {
                        // Base is the receiver (or node start if no receiver)
                        let base_offset = if let Some(recv) = n.receiver() {
                            recv.location().start_offset()
                        } else {
                            n.location().start_offset()
                        };
                        // Use equal_loc for the operator position; fall back to
                        // message_loc (the method name like `[]=`)
                        let op_offset = n
                            .equal_loc()
                            .or(n.message_loc())
                            .map(|l| l.start_offset())
                            .unwrap_or(base_offset);
                        diagnostics.extend(self.check_write(
                            source,
                            base_offset,
                            op_offset,
                            last_arg,
                            width,
                        ));
                    }
                }
            }
        }

        // Call compound writes (obj.x ||= val, obj.x &&= val, obj.x += val)
        if let Some(n) = node.as_call_or_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(
                source,
                base_offset,
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_call_and_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(
                source,
                base_offset,
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_call_operator_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(
                source,
                base_offset,
                n.binary_operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Index compound writes (hash[key] ||= val, hash[key] &&= val, hash[key] += val)
        if let Some(n) = node.as_index_or_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(
                source,
                base_offset,
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_index_and_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(
                source,
                base_offset,
                n.operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_index_operator_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(
                source,
                base_offset,
                n.binary_operator_loc().start_offset(),
                &n.value(),
                width,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(AssignmentIndentation, "cops/layout/assignment_indentation");

    #[test]
    fn single_line_assignment_ignored() {
        let source = b"x = 1\n";
        let diags = run_cop_full(&AssignmentIndentation, source);
        assert!(diags.is_empty());
    }
}
