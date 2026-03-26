use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Rails/Blank - flags code that can be simplified using `Object#blank?` from Active Support.
///
/// ## Investigation findings (2026-03-08)
///
/// **FN root cause (1095 FN):** The `UnlessPresent` pattern was not implemented at all.
/// The config option was read but the variable was prefixed with `_` and never used.
/// RuboCop's `on_if` handler flags `unless foo.present?` (both modifier and block forms)
/// suggesting `if foo.blank?` instead.
///
/// **Fix:** Implemented `UnlessPresent` check via a custom AST visitor in `check_source`.
/// The visitor handles both modifier (`something unless foo.present?`) and block
/// (`unless foo.present? ... end`) forms. Skips unless-with-else when Style/UnlessElse
/// would conflict (conservative: always skip when else clause is present).
///
/// **FP root cause (27 FP, first batch):** Missing `defining_blank?` check. RuboCop skips
/// `!present?` when it appears inside `def blank?` (defining blank? in terms of present?).
/// Nitrocop was incorrectly flagging these as offenses.
///
/// **Fix:** Added parent context tracking in the visitor. When inside a `def blank?` method,
/// `!present?` calls are suppressed.
///
/// **FP root cause (34 FP, second batch, 2026-03-10):** `nil_check_receiver` was matching
/// `!foo` (boolean negation, method name `!`) as a nil-check pattern. This caused
/// `!foo || foo.empty?` to be flagged as NilOrEmpty, but RuboCop's `nil_or_empty?`
/// NodePattern only matches explicit nil checks: `nil?`, `== nil`, `nil ==`. It does NOT
/// match `!foo`. The `!foo` branch was incorrectly added to `nil_check_receiver`.
///
/// **Fix:** Removed the `!` method name branch from `nil_check_receiver`. Only `nil?`,
/// `== nil`, and `nil == foo` are valid nil-check patterns for the NilOrEmpty check.
///
/// ## Investigation (2026-03-14)
///
/// **FP root cause (34 FP, third batch):** `present?` called WITH arguments was flagged.
/// RuboCop's NodePattern `(send (send $_ :present?) :!)` only matches when `present?` has
/// NO arguments. Calls like `!Helpers.present?(value)` or `unless Helpers.present?(value)`
/// use `present?` as a class method with an argument — RuboCop skips these.
///
/// Fix: Added argument count check in `check_not_present` and `check_unless_present`.
///
/// ## Investigation (2026-03-15)
///
/// **FP root cause (6 FP remaining):** Safe navigation calls were flagged.
/// - `unless response&.strip&.present?` → `check_unless_present` was matching `&.present?`
///   but RuboCop's `(send $_ :present?)` only matches `send` not `csend`.
/// - `foo.nil? || foo&.empty?` → `check_nil_or_empty` was matching `&.empty?` right side
///   but RuboCop's `(send $_ :empty?)` only matches `send` not `csend`.
///
/// Fix: Added `call_operator_loc() == &.` check to skip safe navigation calls in
/// `check_unless_present` and `check_nil_or_empty`.
///
/// ## Investigation (2026-03-15, second pass)
///
/// **FP root cause (2 FP remaining):** Pattern match guards `in "div" unless element.at("div").present?`
/// are parsed by Prism as `UnlessNode` inside `InNode`. RuboCop's `on_if` handler does not
/// visit these guard nodes. The cop was incorrectly visiting them.
///
/// Fix: Added `inside_in_node` context tracking. When inside an `InNode`, `check_unless_present`
/// is skipped.
///
/// ## Investigation (2026-03-15, round 3)
///
/// **FN root cause (2 FN):** `nil? || empty?` with implicit receiver (self)
/// was not matched. `nil_check_receiver` required an explicit receiver via
/// `receiver_source` which returns None for receiverless calls.
/// Fix: Changed `nil_check_receiver` return type to `Option<(Option<&[u8]>, &[u8])>` so
/// None receiver (implicit self) is preserved. Updated `check_nil_or_empty` to match
/// when both nil-check and empty? have None receivers (both implicit self).
///
/// **FN root cause (112 FN):** The `!foo || foo.empty?` pattern was not matched by
/// `nil_check_receiver`. RuboCop's `nil_or_empty?` NodePattern includes `(send $_ :!)` as one
/// of the left-side alternatives, meaning `!foo || foo.empty?` is a valid NilOrEmpty offense.
/// The previous fix (2026-03-10) incorrectly removed this pattern, because the old
/// implementation was treating `!foo` as the full left source text instead of extracting `foo`
/// (the receiver of `!`) as the variable to compare with `empty?`'s receiver.
///
/// Fix: Re-added `!` method name to `nil_check_receiver`. Now correctly extracts the receiver
/// of `!` (i.e., `foo` from `!foo`) as the variable, matching RuboCop's `(send $_ :!)` capture.
///
/// ## Investigation (2026-03-19)
///
/// **FP root cause (1 FP):** `!pkey_cols&.present?` uses safe navigation on `present?`.
/// RuboCop's `not_present?` NodePattern `(send (send $_ :present?) :!)` only matches `send`,
/// not `csend`. When `pkey_cols` is nil, `nil&.present?` returns `nil` and `!nil` is `true`,
/// but `pkey_cols&.blank?` would return `nil` (falsy). So the replacement changes semantics.
///
/// Fix: Added `call_operator_loc() == &.` check in `check_not_present` to skip safe navigation.
///
/// ## Investigation (2026-03-26)
///
/// **FN root cause (14 FN):** `check_nil_or_empty` rebuilt the "instead of" text as
/// `{left} || {right}`. That normalized valid `or`/`not` source such as
/// `name.nil? or name.empty?` and `not @params or @params.empty?` into `||` in the
/// diagnostic message. The AST match still fired, but corpus offense matching treated the
/// RuboCop-incompatible message text as a miss.
///
/// Fix: Use the original `OrNode` source slice for the diagnostic's current expression so
/// the message preserves `or`, `||`, `not`, and the exact nested subexpression RuboCop reports.
pub struct Blank;

/// Extract the receiver source text from a CallNode, returning None if absent.
fn receiver_source<'a>(call: &ruby_prism::CallNode<'a>) -> Option<&'a [u8]> {
    call.receiver().map(|r| r.location().as_slice())
}

/// Check if the left side of an OR node matches a nil-check-like pattern:
/// - `foo.nil?` or `nil?` (implicit self)
/// - `foo == nil`
/// - `nil == foo`
/// - `!foo` (boolean negation — RuboCop's `(send $_ :!)` pattern)
///
/// Returns (receiver source bytes option, left side source bytes) if matched.
/// The receiver is `None` when using implicit self (e.g., bare `nil?`).
/// RuboCop's `nil_or_empty?` NodePattern matches all four forms.
fn nil_check_receiver<'a>(node: &ruby_prism::Node<'a>) -> Option<(Option<&'a [u8]>, &'a [u8])> {
    let call = node.as_call_node()?;
    let method = call.name().as_slice();
    let left_src = node.location().as_slice();

    if method == b"nil?" {
        // foo.nil? or bare nil? (implicit self)
        return Some((receiver_source(&call), left_src));
    }

    if method == b"!" {
        // !foo — boolean negation. RuboCop's `(send $_ :!)` captures the receiver.
        return receiver_source(&call).map(|r| (Some(r), left_src));
    }

    if method == b"==" {
        // foo == nil  or  nil == foo
        let recv = call.receiver()?;
        let args = call.arguments()?;
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return None;
        }
        let arg = &arg_list[0];

        if arg.as_nil_node().is_some() {
            // foo == nil → receiver is foo
            return Some((Some(recv.location().as_slice()), left_src));
        }
        if recv.as_nil_node().is_some() {
            // nil == foo → receiver is arg
            return Some((Some(arg.location().as_slice()), left_src));
        }
    }

    None
}

impl Cop for Blank {
    fn name(&self) -> &'static str {
        "Rails/Blank"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let nil_or_empty = config.get_bool("NilOrEmpty", true);
        let not_present = config.get_bool("NotPresent", true);
        let unless_present = config.get_bool("UnlessPresent", true);

        let mut visitor = BlankVisitor {
            cop: self,
            source,
            nil_or_empty,
            not_present,
            unless_present,
            inside_def_blank: false,
            inside_in_node: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct BlankVisitor<'a, 'src> {
    cop: &'a Blank,
    source: &'src SourceFile,
    nil_or_empty: bool,
    not_present: bool,
    unless_present: bool,
    inside_def_blank: bool,
    inside_in_node: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> BlankVisitor<'_, '_> {
    /// Check NilOrEmpty: `foo.nil? || foo.empty?` or `nil? || empty?` (implicit self)
    fn check_nil_or_empty(&mut self, or_node: &ruby_prism::OrNode<'pr>) {
        let left = or_node.left();
        let right = or_node.right();

        if let Some((nil_recv, _left_src)) = nil_check_receiver(&left) {
            // Right side must be `<same>.empty?` — NOT safe navigation (`&.empty?`)
            // RuboCop's NodePattern `(send $_ :empty?)` only matches send, not csend.
            if let Some(right_call) = right.as_call_node() {
                let is_safe_nav = right_call
                    .call_operator_loc()
                    .is_some_and(|loc| loc.as_slice() == b"&.");
                if right_call.name().as_slice() == b"empty?" && !is_safe_nav {
                    let empty_recv = receiver_source(&right_call);
                    // Both receivers must match: both Some with same source, or both None (implicit self)
                    if nil_recv == empty_recv {
                        let loc = or_node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        let current_str =
                            std::str::from_utf8(loc.as_slice()).unwrap_or("nil? || empty?");
                        let message = match nil_recv {
                            Some(recv_bytes) => {
                                let recv_str = std::str::from_utf8(recv_bytes).unwrap_or("object");
                                format!("Use `{recv_str}.blank?` instead of `{current_str}`.")
                            }
                            None => format!("Use `blank?` instead of `{current_str}`."),
                        };
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            message,
                        ));
                    }
                }
            }
        }
    }

    /// Check NotPresent: `!foo.present?`
    fn check_not_present(&mut self, call: &ruby_prism::CallNode<'pr>) {
        if call.name().as_slice() != b"!" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"present?" {
            return;
        }

        // RuboCop's NodePattern `(send (send $_ :present?) :!)` only matches `send`, not `csend`.
        // `!pkey_cols&.present?` uses safe navigation — semantics differ from `!pkey_cols.blank?`
        // because `nil&.present?` returns `nil`, `!nil` is `true`, but `nil&.blank?` returns `nil`.
        if inner_call
            .call_operator_loc()
            .is_some_and(|loc| loc.as_slice() == b"&.")
        {
            return;
        }

        // RuboCop's NodePattern `(send (send $_ :present?) :!)` only matches present? with
        // NO arguments. `!Helpers.present?(value)` (class method style) must not be flagged.
        if inner_call
            .arguments()
            .is_some_and(|a| !a.arguments().is_empty())
        {
            return;
        }

        // Skip !present? inside def blank? (defining blank? in terms of present?)
        if self.inside_def_blank {
            return;
        }

        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `blank?` instead of `!present?`.".to_string(),
        ));
    }

    /// Check UnlessPresent: `unless foo.present?` or `something unless foo.present?`
    fn check_unless_present(&mut self, unless_node: &ruby_prism::UnlessNode<'pr>) {
        // Skip pattern match guards: `in pattern unless condition`
        // In Prism, pattern match guards are represented as UnlessNodes inside InNodes.
        // RuboCop's `on_if` handler does not visit these guards.
        if self.inside_in_node {
            return;
        }

        // Skip unless-with-else (Style/UnlessElse interaction)
        // Conservative: always skip when else clause is present
        if unless_node.else_clause().is_some() {
            return;
        }

        let predicate = unless_node.predicate();
        let pred_call = match predicate.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if pred_call.name().as_slice() != b"present?" {
            return;
        }

        // RuboCop's NodePattern `(send $_ :present?)` only matches send (not csend).
        // `unless obj&.present?` uses safe navigation and must NOT be flagged.
        if pred_call
            .call_operator_loc()
            .is_some_and(|loc| loc.as_slice() == b"&.")
        {
            return;
        }

        // RuboCop's NodePattern `(send $_ :present?)` only matches present? with NO arguments.
        if pred_call
            .arguments()
            .is_some_and(|a| !a.arguments().is_empty())
        {
            return;
        }

        // Build the receiver string for the message
        let recv_str = match pred_call.receiver() {
            Some(r) => {
                let src = r.location().as_slice();
                format!("{}.blank?", std::str::from_utf8(src).unwrap_or("object"))
            }
            None => "blank?".to_string(),
        };

        // Build the "current" string for the message
        let predicate_src =
            std::str::from_utf8(predicate.location().as_slice()).unwrap_or("present?");
        let current = format!("unless {predicate_src}");

        // Determine offense location based on modifier vs block form
        // For modifier form: `something unless foo.present?` → offense on `unless foo.present?`
        // For block form: `unless foo.present?\n...\nend` → offense on `unless foo.present?`
        let unless_loc = unless_node.location();
        let pred_loc = predicate.location();

        // The offense covers from the start of `unless` keyword to the end of the predicate
        // For modifier form, the keyword is in the middle; for block form, it's at the start
        let keyword_loc = unless_node.keyword_loc();
        let offense_start = keyword_loc.start_offset();
        let offense_end = pred_loc.end_offset();

        // Check if this is modifier form by comparing keyword start to node start
        let is_modifier = keyword_loc.start_offset() > unless_loc.start_offset();

        let (line, column) = if is_modifier {
            self.source.offset_to_line_col(offense_start)
        } else {
            // Block form: offense starts at the `unless` keyword (= node start)
            self.source.offset_to_line_col(offense_start)
        };

        // For the offense range length, count from keyword to end of predicate
        let _ = offense_end; // used implicitly via the annotation range

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!("Use `if {recv_str}` instead of `{current}`."),
        ));
    }
}

impl<'pr> Visit<'pr> for BlankVisitor<'_, '_> {
    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        if self.nil_or_empty {
            self.check_nil_or_empty(node);
        }
        // Continue visiting children
        self.visit(&node.left());
        self.visit(&node.right());
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.not_present {
            self.check_not_present(node);
        }
        // Visit children (receiver, arguments, block)
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        if self.unless_present {
            self.check_unless_present(node);
        }
        // Visit children
        self.visit(&node.predicate());
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit(&else_clause.as_node());
        }
    }

    fn visit_in_node(&mut self, node: &ruby_prism::InNode<'pr>) {
        let was_inside = self.inside_in_node;
        self.inside_in_node = true;
        ruby_prism::visit_in_node(self, node);
        self.inside_in_node = was_inside;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let is_blank = node.name().as_slice() == b"blank?";
        let was_inside = self.inside_def_blank;
        if is_blank {
            self.inside_def_blank = true;
        }

        // Visit children: parameters and body
        if let Some(params) = node.parameters() {
            self.visit(&params.as_node());
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }

        self.inside_def_blank = was_inside;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Blank, "cops/rails/blank");
}
