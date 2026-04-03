use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-19)
///
/// Corpus oracle reported FP=4, FN=1.
///
/// FP=4: All 4 FPs from ecleel/hijri repo — `Hijri::Date.today` and `Hijri::DateTime.now`.
/// RuboCop's NodePattern matches `(const {nil? cbase} :Date)` which only accepts bare `Date`
/// or `::Date`, not qualified paths like `Hijri::Date`. Fixed by replacing `constant_name()`
/// (which returns the terminal name) with `is_simple_constant()` which validates the full path.
///
/// FN=1: netzke/netzke-basepack — `to_time_in_current_zone` deprecated method was not detected.
/// Fixed by adding an explicit check for `to_time_in_current_zone` that fires regardless of
/// EnforcedStyle, matching RuboCop's DEPRECATED_METHODS behavior.
///
/// ## Corpus investigation (2026-03-26)
///
/// Corpus oracle reported FP=5, FN=0.
///
/// FP=5: All 5 FPs from cjstewart88/Tubalr — `to_time_in_current_zone` called without an
/// explicit receiver (implicit `self`) inside ActiveSupport's own core_ext/date/ files.
/// RuboCop's `on_send` starts with `return unless node.receiver && ...`, so implicit-self
/// calls are never flagged. Fixed by adding a `call.receiver().is_some()` check before
/// flagging `to_time_in_current_zone` (and `to_time` for the same reason).
pub struct Date;

impl Cop for Date {
    fn name(&self) -> &'static str {
        "Rails/Date"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let style = config.get_str("EnforcedStyle", "flexible");
        let allow_to_time = config.get_bool("AllowToTime", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();

        // `to_time_in_current_zone` is always deprecated, regardless of EnforcedStyle.
        // RuboCop requires a receiver (`node.receiver && ...`), so implicit-self calls
        // like bare `to_time_in_current_zone` inside ActiveSupport are not flagged.
        if method == b"to_time_in_current_zone" && call.receiver().is_some() {
            let msg_loc = match call.message_loc() {
                Some(loc) => loc,
                None => return,
            };
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "`to_time_in_current_zone` is deprecated. Use `in_time_zone` instead.".to_string(),
            ));
            return;
        }

        // In strict mode, also flag `to_time` (requires explicit receiver, same as RuboCop)
        if method == b"to_time" && call.receiver().is_some() && !allow_to_time && style == "strict"
        {
            let msg_loc = match call.message_loc() {
                Some(loc) => loc,
                None => return,
            };
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not use `to_time` in strict mode.".to_string(),
            ));
        }

        if method != b"today" {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        // RuboCop matches `(const {nil? cbase} :Date)` — only bare `Date` or `::Date`,
        // not qualified paths like `Hijri::Date`.
        if !util::is_simple_constant(&recv, b"Date") {
            return;
        }

        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return,
        };
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `Date.current` instead of `Date.today`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Date, "cops/rails/date");
}
