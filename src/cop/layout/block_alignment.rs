use crate::cop::node_type::{CALL_NODE, LAMBDA_NODE, node_type_tag};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks whether the end keywords / closing braces are aligned properly for
/// do..end and {..} blocks.
///
/// ## Corpus investigation findings (2026-03-11)
///
/// Root causes of 1,187 FP:
/// 1. **Trailing-dot method chains** — `find_chain_expression_start` only checked
///    for lines starting with `.` (leading dot) but NOT for lines ending with `.`
///    (trailing dot style). This caused the chain root to not be found, computing
///    wrong `expression_start_indent` and flagging correctly-aligned `end`.
/// 2. **Tab indentation** — `line_indent` only counted spaces, returning 0 for
///    tab-indented lines. But `offset_to_line_col` counts tabs as 1 character,
///    causing a mismatch between computed indent and actual `end` column.
/// 3. **Missing `begins_its_line?` check** — RuboCop skips alignment checks when
///    `end`/`}` is not the first non-whitespace on its line (e.g., `end.select`).
///    nitrocop checked all `end` keywords regardless.
///
/// Root causes of 334 FN:
/// 1. **Brace blocks not checked** — RuboCop checks both `do..end` and `{..}`
///    blocks, but nitrocop only checked `do..end`. Many FNs were misaligned `}`.
///
/// Fixes applied:
/// - `line_indent` now counts both spaces and tabs
/// - `find_chain_expression_start` now handles trailing-dot chains (lines ending with `.`)
/// - Added `begins_its_line` check to skip non-line-beginning closers
/// - Added brace block (`{..}`) checking with same alignment rules
/// - Fixed `start_of_block` style to use do-line indent (not `do` column) per RuboCop spec
///
/// ## Corpus investigation findings (2026-03-14)
///
/// Root causes of remaining 411 FP:
/// 1. **String concatenation `+` continuation** — Lines ending with `+` (common in
///    RSpec multiline descriptions like `it "foo " + "bar" do`) were not recognized
///    as expression continuations. `find_chain_expression_start` stopped too early,
///    computing wrong `expression_start_indent` and flagging correctly-aligned `end`.
///    Fixed by adding `+` to the continuation character set.
///
/// Root causes of remaining 103 FN:
/// 1. **Assignment RHS alignment accepted** — `find_call_expression_col` walked
///    backward from `do`/`{` to find the call expression start, but stopped at the
///    RHS of assignments (e.g., `answer = prompt.select do`). This made `call_expr_col`
///    point to `prompt` instead of `answer`, causing nitrocop to accept `end` aligned
///    with the RHS when RuboCop requires alignment with the LHS variable.
///    Fixed by adding `skip_assignment_backward` to walk through `=`/`+=`/`||=`/etc.
///    to find the LHS variable.
///
/// ## Corpus investigation findings (2026-03-18)
///
/// Root causes of remaining 176 FP:
/// 1. **Multiline string literals** — The line-based heuristic `find_chain_expression_start`
///    could not detect string literals spanning multiple lines without explicit continuation
///    markers (e.g., `it "long desc\n    continued" do`). This caused the expression start
///    to be computed from the wrong line.
/// 2. **Comment lines between continuations** — Comment lines interleaved in multi-line
///    method calls (e.g., RSpec `it` with keyword args after comments) broke the backward
///    line walk.
///
/// Root causes of remaining 55 FN:
/// 1. **Over-eager backward walk** — `find_chain_expression_start` walked through unclosed
///    brackets into outer expressions (e.g., from `lambda{|env|` through `show_status(` into
///    `req = ...`), computing an expression indent that matched the misaligned closer.
///
/// Fix: Replaced `BLOCK_NODE` with `CALL_NODE` dispatch. The CallNode's `location()` in
/// Prism spans the entire expression including receiver chains, giving the exact expression
/// start without heuristic line-based backward walking. This eliminates multiline string,
/// comment interleaving, and bracket-balance issues in one structural change.
/// Replaced `find_chain_expression_start` with `find_operator_continuation_start` which
/// only walks through `||`, `&&`, `<<`, `+` operators (not brackets/commas/backslashes),
/// preventing over-eager backward walking that caused false negatives.
///
/// ## Corpus investigation findings (2026-03-18, round 2)
///
/// Root causes of remaining 16 FP:
/// 1. **Chained blocks in assignment context** — `response = stub_comms do ... end.check_request do`
///    where `end` at col N aligns with the method call (`stub_comms`) but the assignment LHS
///    (`response`) is at a different column. The old code skipped `call_start_col` when
///    `assignment_col.is_some()`, preventing recognition of valid intermediate alignment.
///    Fixed by accepting `call_start_col` when the closer is chained (`.method` or `&.method`
///    follows `end`/`}`).
/// 2. **`&&`/`||` on same line as `do`/`{`** — `a && b.each do ... end` where `end` aligns
///    with the LHS of the `&&` expression. Added `find_same_line_operator_lhs` to detect
///    binary operators before the CallNode on the same line.
///
/// Root causes of remaining 34 FN:
/// 1. **Lambda/proc blocks not checked** — `-> { }` and `-> do end` produce `LambdaNode` in
///    Prism, not `CallNode`. The cop only dispatched on `CALL_NODE`. Added `LAMBDA_NODE`
///    dispatch with `check_lambda_alignment` method.
/// 2. **`do_col` incorrectly accepted as alignment target** — The column of the `do`/`{`
///    keyword itself was accepted in "either" mode, but RuboCop only accepts the indent
///    of the do-line (`indentation_of_do_line`) and the expression start column. Removing
///    `do_col` from accepted targets fixes FNs like `Hash.new do ... end` where `end` at
///    the `do` column was incorrectly accepted.
/// 3. **Lambda/proc blocks not checked** — `-> { }` and `-> do end` produce `LambdaNode` in
///    Prism, not `CallNode`. The cop only dispatched on `CALL_NODE`. Added `LAMBDA_NODE`
///    dispatch with `check_lambda_alignment` method.
/// 4. **Incorrect no_offense fixture cases** — Several fixture cases had `}` aligned with
///    the method call column (not the expression/line start), which RuboCop would flag.
///    Removed factually incorrect cases from no_offense.rb.
///
/// ## Corpus investigation findings (2026-03-19)
///
/// Root causes of remaining 6 FP:
/// 1. **Next-line dot chain** (ubicloud, 2 FP) — `}` followed by newline + `.sort_by` was
///    not detected as a chained closer because `is_closer_chained` only checked for `.`
///    immediately after the closer. Extended to check the next non-empty line for leading `.`.
/// 2. **`&&`/`||` with complex LHS** (forem, 1 FP) — `if x == "str" && y.each do ... end`
///    where `find_same_line_operator_lhs` couldn't walk backward through string literals
///    and `==` in the LHS. Made the backward walk more permissive (handles quotes, operators).
/// 3. **Multiline assignment on previous line** (pivotal, 1 FP) — `a, b =\n  stdout\n  .reduce do`
///    where `find_assignment_lhs_col` only checked the same line as the CallNode. Extended
///    to check the previous line when the call starts at line indent and prev line ends with `=`.
/// 4. **Paren/rescue context** (automaticmode, 1 FP) — `(svg = IO.popen(...) { } rescue false)`.
///    Not fixed; requires AST parent walk.
/// 5. **Splat method arg deep indentation** (flyerhzm, 1 FP) — `*descendants.map { ... }`.
///    Not fixed; requires AST parent walk.
///
/// Root causes of remaining 17 FN:
/// 1. **`expression_start_indent` too permissive** (Arachni, 6 FN + seyhunak, 3 FN + others) —
///    When a block's call expression is mid-line (e.g., inside parens like `expect(auditable.audit(...)
///    do`), the line indent matches the outer context, not the call expression. Guarded
///    `expression_start_indent` to only be accepted when `call_start_col == expression_start_indent`
///    (i.e., the call starts at the line's indent position).
/// 2. **`%` not in `find_call_expression_col` chars** (randym, 2 FN + floere, 1 FN) —
///    `%w(...)` percent literals weren't fully walked backward. Added `%` to accepted characters.
/// 3. **Lambda `call_expr_col` accepting `{` column** (refinery, 1 FN) — For lambda blocks,
///    `find_call_expression_col` gave the `{`/`do` position rather than `->`, causing `}`
///    aligned with `{` to be accepted. Removed `call_expr_col` from lambda alignment check.
/// 4. **Chained `.to_json` accepted in assignment** (diaspora, 1 FN) — Not fixed; chained
///    closer heuristic accepts `call_start_col` which matches the RHS call. Requires AST walk.
///
/// Remaining gaps: 2 FP (paren/rescue, splat-arg) + 1 FN (chained closer in assignment).
///
/// ## Corpus investigation findings (2026-03-19, round 3)
///
/// Root causes of remaining 10 FP:
/// 1. **`:` in bracket-key LHS** (vagrant-parallels x2, hashicorp/vagrant, JEG2/highline,
///    peritor/webistrano — 5 FP) — `env[:machine].id = expr do` or `entry[:phone] = ask(...) do`.
///    `skip_assignment_backward` LHS walk didn't handle `:` (symbol literal prefix inside
///    brackets), stopping at `:machine` instead of walking through to `env`. Fixed by adding
///    `:` to accepted chars and balanced paren/bracket handling in the LHS walk.
/// 2. **`<<` not handled as same-line operator** (docuseal, openstreetmap — 2 FP) —
///    `acc << expr do ... end` or `lists << tag.ul(...) do`. The `<<` shovel operator wasn't
///    recognized by `find_same_line_operator_lhs`, so `acc`'s column wasn't accepted as
///    alignment target. Added `<<` to the same-line operator check.
/// 3. **Parens not handled in LHS walk** (openstreetmap, opf/openproject — 1 FP) —
///    `RequestStore.store[key(work_package)] = value do` where `(` in the LHS stopped the
///    backward walk. Added balanced paren/bracket handling to `skip_assignment_backward`.
/// 4. Existing unfixable: paren/rescue (automaticmode, 1 FP), splat-arg (flyerhzm, 1 FP).
///
/// Root causes of remaining 7 FN:
/// 1. **Cross-line single assignment accepted** (ankane/blazer, fog, jruby/warbler — 3 FN) —
///    `@connection_model =\n  Class.new(...) do ... end` at col 8. `find_assignment_lhs_col`
///    walked to the previous line and found the assignment LHS. But RuboCop's
///    `disqualified_parent?` stops at cross-line parents (except masgn). Fixed by restricting
///    cross-line assignment detection to multi-assignment (masgn) only (detected by comma in LHS).
/// 2. **Cross-line `||`/`&&` accepted as alignment target** (sharetribe — 1 FN) —
///    `accepted_states.empty? ||\n  accepted_states.any? do ... end` at col 6.
///    `find_operator_continuation_start` accepted the indent of the `||` LHS line. But RuboCop's
///    `disqualified_parent?` stops at cross-line parents. Fixed by removing
///    `find_operator_continuation_start` entirely — cross-line operator continuations are
///    not valid alignment targets.
/// 3. **Cross-line `<<` no longer accepted** (trogdoro — 1 FN) — `threads <<\n  Thread::new(...)
///    do ... end` at col 10 matched `threads <<` line indent via `operator_continuation_indent`.
///    Removing that function fixed this FN.
/// 4. **Cross-line single assignment no longer accepted** (sup-heliotrope — 1 FN) —
///    `@files =\n  begin...end.map do ... end` at col 4. The cross-line `@files =` was
///    previously accepted; masgn restriction now rejects it.
/// 5. Existing unfixable: chained `.to_json` in assignment (diaspora, 1 FN).
///
/// Remaining gaps: 2 FP (paren/rescue, splat-arg) + 1 FN (chained closer in assignment).
///
/// ## Corpus investigation findings (2026-03-20)
///
/// Root causes of the final oracle-known gaps:
/// 1. **Rescue modifier wrapper** (automaticmode, 1 FP) — `foo { ... } rescue false`
///    should stop the ancestor walk at the current block expression, so the closer
///    aligns with the block call start rather than the outer assignment LHS.
/// 2. **Splat wrapper** (flyerhzm, 1 FP) — `wrap *items.map { ... }` aligns the
///    closer with the `*` column because RuboCop stops at the `splat` ancestor.
/// 3. **Plain chained call in assignment** (diaspora, 1 FN) — `result = items.map { ... }.to_json`
///    must NOT accept the inner call start. RuboCop walks through the normal send
///    chain to the assignment, so the closer aligns with the assignment LHS.
///
/// Fixes applied:
/// - Added `find_same_line_splat_col` so splat-wrapped block calls align to `*`
/// - Replaced the broad chained-closer escape with `accept_intermediate_call_start`,
///   which only keeps the inner call start for rescue wrappers, safe-navigation
///   chains (`&.`), and chained calls that immediately open another block
///
/// Verification:
/// - `cargo test --lib -- block_alignment` passes with new fixture coverage for all
///   three patterns
/// - `scripts/verify-cop-locations.py Layout/BlockAlignment` reports all CI-known
///   FP/FN fixed
/// - `scripts/check-cop.py Layout/BlockAlignment --verbose --rerun` still reports
///   15 excess in local batch `--corpus-check` mode, but a direct per-repo
///   nitrocop vs RuboCop sweep over all 188 active repos shows 0 count delta on
///   the 180 repos that were locally comparable. The 8 remaining repos failed
///   local RuboCop/json validation (`devdocs`, `jruby`, and 6 repos with local
///   JSON/tooling issues), so the residual batch excess is likely validation noise
///   rather than a confirmed cop-logic mismatch.
///
/// ## Corpus investigation findings (2026-03-30)
///
/// Root causes of the remaining FN:
/// 1. **Wrapper ancestors were still inferred from bytes** — same-line `||`, `<<`,
///    splat wrappers, and receiver chains like `end.to_a` or `end.sort_by { ... }`
///    need RuboCop's AST ancestor walk, not `call_expr_col` heuristics. Those
///    heuristics accepted the RHS call start (`proxy_target`, `sequence`,
///    `ThreadPoolJob`, etc.) when RuboCop aligns to the wrapper expression start.
/// 2. **Outer block barriers were invisible** — when a misaligned block feeds a
///    chained call that opens another block (`end.sort_by { ... }`), RuboCop stops
///    at that parent call. Prism stores the outer block as a child of the parent
///    call, so nitrocop must stop explicitly after stepping into that wrapper.
/// 3. **Lambda mid-line indentation was accepted** — `-> (...) do` accepted the
///    line indent of the surrounding `scope ...` call instead of only the `->`
///    column or the `do`/`{` line indent.
///
/// Fix: replaced wrapper-target guessing with a Prism visitor that walks actual
/// ancestors for block-bearing calls, mirroring RuboCop's allowed wrappers
/// (`assignment`, `and/or`, `splat`, `<<`, receiver chains) and stopping after
/// parent calls that open their own blocks. Lambda alignment now only accepts the
/// lambda start column or the `do`/`{` line indent.
pub struct BlockAlignment;

impl Cop for BlockAlignment {
    fn name(&self) -> &'static str {
        "Layout/BlockAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, LAMBDA_NODE]
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
        let style = AlignmentStyle::from_config(config);
        let mut visitor = BlockAlignmentVisitor {
            cop: self,
            source,
            style,
            diagnostics,
            ancestors: Vec::new(),
        };
        visitor.visit(&parse_result.node());
    }

    fn check_node(
        &self,
        _source: &SourceFile,
        _node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        _diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }
}

#[derive(Clone, Copy)]
enum AlignmentStyle {
    Either,
    StartOfBlock,
    StartOfLine,
}

struct BlockAlignmentVisitor<'a, 'pr> {
    cop: &'a BlockAlignment,
    source: &'a SourceFile,
    style: AlignmentStyle,
    diagnostics: &'a mut Vec<Diagnostic>,
    ancestors: Vec<ruby_prism::Node<'pr>>,
}

impl<'pr> Visit<'pr> for BlockAlignmentVisitor<'_, 'pr> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.ancestors.push(node);
    }

    fn visit_branch_node_leave(&mut self) {
        self.ancestors.pop();
    }

    fn visit_leaf_node_enter(&mut self, _node: ruby_prism::Node<'pr>) {}

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(block_node) = node.block().and_then(|block| block.as_block_node()) {
            self.cop.check_call_alignment(
                self.source,
                node,
                &block_node,
                self.style,
                &self.ancestors,
                self.diagnostics,
            );
        }

        ruby_prism::visit_call_node(self, node);
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.cop
            .check_lambda_alignment(self.source, node, self.style, self.diagnostics);
        ruby_prism::visit_lambda_node(self, node);
    }
}

impl BlockAlignment {
    fn check_call_alignment<'pr>(
        &self,
        source: &SourceFile,
        _call_node: &ruby_prism::CallNode<'pr>,
        block_node: &ruby_prism::BlockNode<'pr>,
        style: AlignmentStyle,
        ancestors: &[ruby_prism::Node<'pr>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let closing_loc = block_node.closing_loc();
        let closing_slice = closing_loc.as_slice();
        let is_do_end = closing_slice == b"end";
        let is_brace = closing_slice == b"}";
        if !is_do_end && !is_brace {
            return;
        }

        let bytes = source.as_bytes();
        if !begins_its_line(bytes, closing_loc.start_offset()) {
            return;
        }

        let opening_loc = block_node.opening_loc();
        let (opening_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (end_line, end_col) = source.offset_to_line_col(closing_loc.start_offset());
        if end_line == opening_line {
            return;
        }

        let start_of_line_indent = line_indent(bytes, opening_loc.start_offset());
        let start_node = block_alignment_target(source, ancestors);
        let (_, start_col) = source.offset_to_line_col(start_node.location().start_offset());
        let close_word = if is_brace { "`}`" } else { "`end`" };
        let open_word = if is_brace { "`{`" } else { "`do`" };
        let misaligned = match style {
            AlignmentStyle::StartOfBlock => end_col != start_of_line_indent,
            AlignmentStyle::StartOfLine => end_col != start_col,
            AlignmentStyle::Either => end_col != start_col && end_col != start_of_line_indent,
        };

        if misaligned {
            diagnostics.push(self.diagnostic(
                source,
                end_line,
                end_col,
                diagnostic_message(style, close_word, open_word),
            ));
        }
    }

    fn check_lambda_alignment(
        &self,
        source: &SourceFile,
        lambda_node: &ruby_prism::LambdaNode<'_>,
        style: AlignmentStyle,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let closing_loc = lambda_node.closing_loc();
        let closing_slice = closing_loc.as_slice();
        let is_do_end = closing_slice == b"end";
        let is_brace = closing_slice == b"}";
        if !is_do_end && !is_brace {
            return;
        }

        let bytes = source.as_bytes();
        if !begins_its_line(bytes, closing_loc.start_offset()) {
            return;
        }

        let opening_loc = lambda_node.opening_loc();
        let (opening_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (end_line, end_col) = source.offset_to_line_col(closing_loc.start_offset());
        if end_line == opening_line {
            return;
        }

        let start_of_line_indent = line_indent(bytes, opening_loc.start_offset());
        let (_, lambda_start_col) =
            source.offset_to_line_col(lambda_node.location().start_offset());
        let close_word = if is_brace { "`}`" } else { "`end`" };
        let open_word = if is_brace { "`{`" } else { "`do`" };
        let misaligned = match style {
            AlignmentStyle::StartOfBlock => end_col != start_of_line_indent,
            AlignmentStyle::StartOfLine => end_col != lambda_start_col,
            AlignmentStyle::Either => {
                end_col != lambda_start_col && end_col != start_of_line_indent
            }
        };

        if misaligned {
            diagnostics.push(self.diagnostic(
                source,
                end_line,
                end_col,
                diagnostic_message(style, close_word, open_word),
            ));
        }
    }
}

fn block_alignment_target<'pr>(
    source: &SourceFile,
    ancestors: &'pr [ruby_prism::Node<'pr>],
) -> &'pr ruby_prism::Node<'pr> {
    let mut current = ancestors
        .last()
        .expect("block alignment visitor always pushes the current call node");
    let mut stop_after_step = false;

    for parent in ancestors.iter().rev().skip(1) {
        if stop_after_step
            || disqualified_parent(source, parent, current)
            || !is_alignment_wrapper(parent, current)
        {
            break;
        }

        current = parent;
        stop_after_step = current
            .as_call_node()
            .is_some_and(|call| has_block_node(&call));
    }

    current
}

fn disqualified_parent(
    source: &SourceFile,
    parent: &ruby_prism::Node<'_>,
    current: &ruby_prism::Node<'_>,
) -> bool {
    let (parent_line, _) = source.offset_to_line_col(parent.location().start_offset());
    let (current_line, _) = source.offset_to_line_col(current.location().start_offset());
    parent_line != current_line && parent.as_multi_write_node().is_none()
}

fn is_alignment_wrapper(parent: &ruby_prism::Node<'_>, current: &ruby_prism::Node<'_>) -> bool {
    if is_assignment_wrapper(parent, current)
        || parent.as_and_node().is_some()
        || parent.as_or_node().is_some()
    {
        return true;
    }

    if let Some(splat_node) = parent.as_splat_node() {
        return splat_node
            .expression()
            .is_some_and(|expression| same_node(&expression, current));
    }

    let Some(call_node) = parent.as_call_node() else {
        return false;
    };

    if call_node.name().as_slice() == b"<<" && arguments_include(call_node.arguments(), current) {
        return true;
    }

    if (call_node.equal_loc().is_some() || call_node.is_attribute_write())
        && arguments_include(call_node.arguments(), current)
    {
        return true;
    }

    !call_node.is_safe_navigation()
        && call_node.name().as_slice() != b"[]"
        && call_node
            .receiver()
            .is_some_and(|receiver| same_node(&receiver, current))
}

fn is_assignment_wrapper(parent: &ruby_prism::Node<'_>, current: &ruby_prism::Node<'_>) -> bool {
    macro_rules! value_wrapper {
        ($cast:ident) => {
            if let Some(node) = parent.$cast() {
                return same_node(&node.value(), current);
            }
        };
    }

    value_wrapper!(as_call_and_write_node);
    value_wrapper!(as_call_operator_write_node);
    value_wrapper!(as_call_or_write_node);
    value_wrapper!(as_class_variable_and_write_node);
    value_wrapper!(as_class_variable_operator_write_node);
    value_wrapper!(as_class_variable_or_write_node);
    value_wrapper!(as_class_variable_write_node);
    value_wrapper!(as_constant_and_write_node);
    value_wrapper!(as_constant_operator_write_node);
    value_wrapper!(as_constant_or_write_node);
    value_wrapper!(as_constant_path_and_write_node);
    value_wrapper!(as_constant_path_operator_write_node);
    value_wrapper!(as_constant_path_or_write_node);
    value_wrapper!(as_constant_path_write_node);
    value_wrapper!(as_constant_write_node);
    value_wrapper!(as_global_variable_and_write_node);
    value_wrapper!(as_global_variable_operator_write_node);
    value_wrapper!(as_global_variable_or_write_node);
    value_wrapper!(as_global_variable_write_node);
    value_wrapper!(as_index_and_write_node);
    value_wrapper!(as_index_operator_write_node);
    value_wrapper!(as_index_or_write_node);
    value_wrapper!(as_instance_variable_and_write_node);
    value_wrapper!(as_instance_variable_operator_write_node);
    value_wrapper!(as_instance_variable_or_write_node);
    value_wrapper!(as_instance_variable_write_node);
    value_wrapper!(as_local_variable_and_write_node);
    value_wrapper!(as_local_variable_operator_write_node);
    value_wrapper!(as_local_variable_or_write_node);
    value_wrapper!(as_local_variable_write_node);
    value_wrapper!(as_multi_write_node);

    false
}

fn has_block_node(call_node: &ruby_prism::CallNode<'_>) -> bool {
    call_node
        .block()
        .and_then(|block| block.as_block_node())
        .is_some()
}

fn arguments_include(
    arguments: Option<ruby_prism::ArgumentsNode<'_>>,
    current: &ruby_prism::Node<'_>,
) -> bool {
    arguments.is_some_and(|arguments_node| {
        arguments_node
            .arguments()
            .iter()
            .any(|argument| same_node(&argument, current))
    })
}

fn same_node(left: &ruby_prism::Node<'_>, right: &ruby_prism::Node<'_>) -> bool {
    node_type_tag(left) == node_type_tag(right)
        && left.location().start_offset() == right.location().start_offset()
        && left.location().end_offset() == right.location().end_offset()
}

fn diagnostic_message(style: AlignmentStyle, close_word: &str, open_word: &str) -> String {
    match style {
        AlignmentStyle::StartOfBlock => format!("Align {close_word} with {open_word}."),
        AlignmentStyle::StartOfLine | AlignmentStyle::Either => {
            format!("Align {close_word} with the start of the line where the block is defined.")
        }
    }
}

impl AlignmentStyle {
    fn from_config(config: &CopConfig) -> Self {
        match config.get_str("EnforcedStyleAlignWith", "either") {
            "start_of_block" => Self::StartOfBlock,
            "start_of_line" => Self::StartOfLine,
            _ => Self::Either,
        }
    }
}

/// Check if a byte offset is at the beginning of its line (only whitespace before it).
/// Matches RuboCop's `begins_its_line?` helper.
fn begins_its_line(bytes: &[u8], offset: usize) -> bool {
    let mut pos = offset;
    while pos > 0 && bytes[pos - 1] != b'\n' {
        pos -= 1;
        if bytes[pos] != b' ' && bytes[pos] != b'\t' {
            return false;
        }
    }
    true
}

/// Get the indentation (number of leading whitespace characters) for the line
/// containing the given byte offset. Counts both spaces and tabs as 1 character
/// each to match `offset_to_line_col` which uses character (codepoint) offsets.
fn line_indent(bytes: &[u8], offset: usize) -> usize {
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let mut indent = 0;
    while line_start + indent < bytes.len()
        && (bytes[line_start + indent] == b' ' || bytes[line_start + indent] == b'\t')
    {
        indent += 1;
    }
    indent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(BlockAlignment, "cops/layout/block_alignment");

    #[test]
    fn brace_block_no_offense() {
        let source = b"items.each { |x|\n  puts x\n}\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn start_of_block_style() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleAlignWith".into(),
                serde_yml::Value::String("start_of_block".into()),
            )]),
            ..CopConfig::default()
        };
        // In start_of_block style, `end` must align with the do-line indent
        // (first non-ws on the do-line), not the `do` keyword column.
        // For `items.each do |x|`, do-line indent = 0, so end at col 0 is fine.
        let src = b"items.each do |x|\n  puts x\nend\n";
        let diags = run_cop_full_with_config(&BlockAlignment, src, config.clone());
        assert!(
            diags.is_empty(),
            "start_of_block: end at col 0 matches do-line indent 0. Got: {:?}",
            diags
        );

        // But end at col 2 should be flagged (doesn't match do-line indent 0)
        let src2 = b"items.each do |x|\n  puts x\n  end\n";
        let diags2 = run_cop_full_with_config(&BlockAlignment, src2, config.clone());
        assert_eq!(
            diags2.len(),
            1,
            "start_of_block should flag end at col 2 (doesn't match do-line indent 0)"
        );

        // Chained: .each do at col 2, end should align at col 2
        let src3 = b"foo.bar\n  .each do\n    baz\n  end\n";
        let diags3 = run_cop_full_with_config(&BlockAlignment, src3, config.clone());
        assert!(
            diags3.is_empty(),
            "start_of_block: end at col 2 matches .each do line indent. Got: {:?}",
            diags3
        );

        // Chained: .each do at col 2, end at col 0 should flag
        let src4 = b"foo.bar\n  .each do\n    baz\nend\n";
        let diags4 = run_cop_full_with_config(&BlockAlignment, src4, config);
        assert_eq!(
            diags4.len(),
            1,
            "start_of_block: end at col 0 doesn't match .each do line indent 2"
        );
    }

    // FP fix: trailing-dot method chains
    #[test]
    fn no_offense_trailing_dot_chain() {
        let source =
            b"all_objects.flat_map { |o| o }.\n  uniq(&:first).each do |a, o|\n  process(a, o)\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Trailing dot chain: end should align with chain root. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_trailing_dot_chain_indented() {
        let source = b"def foo\n  objects.flat_map { |o| o }.\n    uniq.each do |item|\n    process(item)\n  end\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Indented trailing dot chain: end at col 2 matches chain start at col 2. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_trailing_dot_multi_line() {
        let source = b"  records.\n    where(active: true).\n    order(:name).each do |r|\n    process(r)\n  end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Multi trailing dot: end at col 2 matches chain root at col 2. Got: {:?}",
            diags
        );
    }

    // FP fix: tab indentation
    #[test]
    fn no_offense_tab_indented_block() {
        let source = b"if true\n\titems.each do\n\t\tputs 'hello'\n\tend\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Tab-indented block should not be flagged. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_tab_indented_assignment_block() {
        let source = b"\tvariable = test do |x|\n\t\tx.to_s\n\tend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Tab-indented assignment block should not be flagged. Got: {:?}",
            diags
        );
    }

    // FP fix: begins_its_line check
    #[test]
    fn fp_end_not_beginning_its_line() {
        // end.select is at start of line (after whitespace) but has continuation
        // The first block's end should not be checked since it has .select after it
        let source = b"def foo(bar)\n  bar.get_stuffs\n      .reject do |stuff|\n        stuff.long_expr\n      end.select do |stuff|\n        stuff.other\n      end\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Should not flag end that doesn't begin its line. Got: {:?}",
            diags
        );
    }

    // FN fix: brace block misalignment
    #[test]
    fn offense_brace_block_misaligned() {
        let source = b"test {\n  stuff\n  }\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "Misaligned brace block should be flagged. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_brace_block_aligned() {
        let source = b"test {\n  stuff\n}\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Aligned brace block should not be flagged. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_brace_block_not_beginning_line() {
        let source = b"scope :bar, lambda { joins(:baz)\n                     .distinct }\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "closing brace not beginning its line should not be flagged"
        );
    }

    // Other patterns from RuboCop spec
    #[test]
    fn no_offense_variable_assignment() {
        let source = b"variable = test do |ala|\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end aligned with variable start. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_op_asgn() {
        let source = b"rb += files.select do |file|\n  file << something\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(diags.is_empty(), "end aligned with rb. Got: {:?}", diags);
    }

    #[test]
    fn no_offense_logical_operand() {
        let source = b"(value.is_a? Array) && value.all? do |subvalue|\n  type_check_value(subvalue, array_type)\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end aligns with expression start. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_send_shovel() {
        let source = b"parser.children << lambda do |token|\n  token << 1\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end aligns with parser.children. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_chain_pretty_alignment() {
        let source = b"def foo(bar)\n  bar.get_stuffs\n      .reject do |stuff|\n        stuff.long_expr\n      end\n      .select do |stuff|\n        stuff.other\n      end\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end at col 6 matches do-line indent. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_next_line_assignment() {
        let source = b"variable =\n  a_long_method do |v|\n    v.foo\n  end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end aligns with a_long_method. Got: {:?}",
            diags
        );
    }

    // FP fix: string concatenation with + across lines (RSpec-style descriptions)
    #[test]
    fn no_offense_plus_continuation() {
        // it "something " + "else" do ... end
        let source = b"it \"should convert \" +\n    \"correctly\" do\n  run_test\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Plus continuation: end at col 0 matches chain root. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_plus_continuation_describe() {
        // describe with + continuation spanning 3 lines
        let source = b"describe User, \"when created \" +\n    \"with issues\" do\n  it \"works\" do\n    true\n  end\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Describe + continuation: end at col 0 matches describe. Got: {:?}",
            diags
        );
    }

    // FN fix: end aligns with RHS of assignment instead of LHS
    #[test]
    fn offense_end_aligns_with_rhs() {
        // answer = prompt.select do ... end — end should align with answer, not prompt
        let source =
            b"answer = prompt.select do |menu|\n           menu.choice \"A\"\n         end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "end at col 9 aligns with prompt (RHS) not answer (LHS). Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_assignment_end_aligns_with_lhs() {
        // answer = prompt.select do ... end — end at col 0 aligns with answer (LHS)
        let source = b"answer = prompt.select do |menu|\n  menu.choice \"A\"\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end at col 0 matches answer (LHS). Got: {:?}",
            diags
        );
    }

    // Ensure hash value blocks still work (not regressed by assignment fix)
    #[test]
    fn no_offense_hash_value_block() {
        let source = b"def generate\n  {\n    data: items.map do |item|\n            item.to_s\n          end,\n  }\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Hash value: end aligns with items.map. Got: {:?}",
            diags
        );
    }

    // Block inside parentheses (like expect(...))
    #[test]
    fn no_offense_block_in_parens() {
        let source = b"expect(arr.all? do |o|\n         o.valid?\n       end)\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Block in parens: end at col 7 matches arr.all?. Got: {:?}",
            diags
        );
    }

    // FP fix: chained blocks with end aligning with method call (active_merchant)
    #[test]
    fn fp_chained_block_end_aligns_with_method() {
        // response = stub_comms do ... end.check_request do ... end.respond_with(...)
        // The first end at col 11 aligns with stub_comms at col 11
        let source = b"response = stub_comms do\n             @gateway.verify(@credit_card, @options)\n           end.check_request do |_endpoint, data, _headers|\n  assert_match(/pattern/, data)\nend.respond_with(response)\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Chained blocks: end at col 11 matches stub_comms. Got: {:?}",
            diags
        );
    }

    // Brace block } aligned with call start in chained context
    #[test]
    fn no_offense_brace_chained() {
        // } is followed by .sort_by (chained), so call_start_col is accepted
        let source = b"victims = replicas.select {\n            !(it.destroy_set?)\n          }.sort_by { |r| r.created_at }\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Chained brace: }} at col 10 matches call. Got: {:?}",
            diags
        );
    }

    // FN fix: Hash.new with block end misaligned (jruby)
    #[test]
    fn fn_hash_new_block_end_misaligned() {
        let source = b"NF_HASH_D = Hash.new do |hash, key|\n                       hash.shift if hash.length>MAX_HASH_LENGTH\n                       hash[key] = nfd_one(key)\n                     end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "Hash.new end at col 21 misaligned with NF_HASH_D at col 0. Got: {:?}",
            diags
        );
    }

    // FP: } followed by newline + .sort_by (chained via next-line dot)
    #[test]
    fn fp_brace_chained_next_line_dot() {
        // } at col 16, followed by \n        .sort_by
        // RuboCop accepts this — the block is chained
        let source = b"      victims = replicas.select {\n                  !(it.destroy_set? || it.strand.label == \"destroy\")\n                }\n        .sort_by { |r| r.created_at }\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "FP: brace chained via next-line dot should not be flagged. Got: {:?}",
            diags
        );
    }

    // Known remaining FP: } inside parenthesized expression with rescue modifier
    // (automaticmode__active_workflow, 1 FP). The } aligns with neither the
    // assignment LHS nor the call expression start. RuboCop accepts it through
    // AST parent walk that nitrocop can't replicate with byte-level heuristics.

    // Known remaining FP: } aligned with block body for splat method arg block
    // (flyerhzm__rails_best_practices, 1 FP). Deep indentation method arg pattern
    // where } aligns with the block body, not any standard alignment target.

    // FP: do..end block as part of if condition with &&
    #[test]
    fn fp_do_end_in_if_condition() {
        // if adjustment_type == "removal" && article.tag_list.none? do |tag|
        //      tag.casecmp(tag_name).zero?
        //    end
        let source = b"    if adjustment_type == \"removal\" && article.tag_list.none? do |tag|\n         tag.casecmp(tag_name).zero?\n       end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "FP: do..end in if condition with && should not be flagged. Got: {:?}",
            diags
        );
    }

    // FP: end at col 6 for .reduce block with multiline assignment on previous line
    #[test]
    fn fp_reduce_multiline_assignment() {
        let source = b"    def packages_lines(stdout)\n      packages_lines, last_package_lines =\n        stdout\n        .each_line\n        .map(&:strip)\n        .reject { |line| end_of_package_lines?(line) }\n        .reduce([[], []]) do |(packages_lines, package_lines), line|\n        if start_of_package_lines?(line)\n          packages_lines.push(package_lines) unless package_lines.empty?\n          [packages_lines, [line]]\n        else\n          package_lines.push(line)\n          [packages_lines, package_lines]\n        end\n      end\n\n      packages_lines.push(last_package_lines)\n    end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "FP: end at col 6 matches multiline assignment LHS. Got: {:?}",
            diags
        );
    }

    // FN: end misaligned in multi-arg call with do block (Arachni pattern)
    #[test]
    fn fn_end_misaligned_in_multi_arg_call() {
        // expect(auditable.audit( {},
        //                   format: [...]) do |_, element|
        //     injected << element.affected_input_value
        // end).to be_nil
        // end at col 24, but auditable.audit at col 31 and do-line indent ~42
        let source = b"                        expect(auditable.audit( {},\n                                          format: [ Format::STRAIGHT ] ) do |_, element|\n                            injected << element.affected_input_value\n                        end).to be_nil\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "FN: end at col 24 misaligned in multi-arg call. Got: {:?}",
            diags
        );
    }

    // FN: } misaligned in brace block (seyhunak pattern)
    #[test]
    fn fn_brace_misaligned_deep_block() {
        // have_tag(:div,
        //   with: {class: "alert"}) {
        //     have_tag(:button, ...)
        //   }          <-- } at col 6, but call starts much deeper
        let source = b"      expect(element).to have_tag(:div,\n        with: {class: \"alert\"}) {\n          have_tag(:button,\n            text: \"x\"\n          )\n\n      }\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "FN: brace at col 6 misaligned with have_tag at col 25. Got: {:?}",
            diags
        );
    }

    // FN: end misaligned off by 1 (randym pattern)
    #[test]
    fn fn_end_misaligned_by_one() {
        // %w(...).each do |attr|
        //    body
        //  end         <-- end at col 4, but %w at col 3
        let source = b"   %w(param1 param2).each do |attr|\n      assert_raise(ArgumentError) { @dn.send(attr) }\n    end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "FN: end at col 4 misaligned with %w at col 3. Got: {:?}",
            diags
        );
    }

    // Known remaining FN: } misaligned in assignment context with chained closer
    // (diaspora, 1 FN). `json = bob.contacts.map { ... }.to_json` — the chained
    // `.to_json` causes nitrocop to accept call_start_col as alignment target.
    // RuboCop uses AST parent walk to resolve through the chain to the assignment.

    // FN: end misaligned in accepted_states.any? (sharetribe pattern)
    #[test]
    fn fn_end_misaligned_any_block() {
        let source = b"        accepted_states.any? do |(status, reason)|\n        if reason.nil?\n          payment[:payment_status] == status\n        end\n          end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "FN: end at col 10 misaligned with accepted_states at col 8. Got: {:?}",
            diags
        );
    }

    // FN: end misaligned by 2 in Thread::new block (trogdoro pattern)
    #[test]
    fn fn_thread_new_block_misaligned() {
        let source = b"            Thread::new(iodat, main) do |iodat, main|\n              process(iodat)\n          end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "FN: end at col 10 misaligned with Thread at col 12. Got: {:?}",
            diags
        );
    }

    // FN: end misaligned in combos block (bloom-lang pattern)
    #[test]
    fn fn_combos_block_misaligned() {
        let source = b"    result <= (sem_hist * use_tiebreak * explicit_tc).combos(sem_hist.from => use_tiebreak.from,\n                                                             sem_hist.to => explicit_tc.from,\n                                                             sem_hist.from => explicit_tc.to) do |s,t,e|\n      [s.to, t.to]\n    end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "FN: end at col 4 misaligned with result or (sem_hist. Got: {:?}",
            diags
        );
    }

    // FN: } misaligned lambda block (refinery pattern)
    #[test]
    fn fn_lambda_brace_misaligned() {
        let source = b"  ->{\n    page.within_frame do\n      select_upload\n    end\n    }\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "FN: brace at col 4 misaligned with -> at col 2. Got: {:?}",
            diags
        );
    }

    // FP: do on continuation line of multi-line method call with assignment
    // env[:machine].id = env[:machine].provider.driver.clone_vm(
    //   env[:clone_id], options) do |progress|
    //   ...
    // end   <-- end at col 10, aligns with assignment LHS env[:machine].id
    #[test]
    fn fp_do_on_continuation_line_with_assignment() {
        let source = b"          env[:machine].id = env[:machine].provider.driver.clone_vm(\n            env[:clone_id], options) do |progress|\n            env[:ui].clear_line\n          end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "FP: end at col 10 aligns with LHS env[:machine].id at col 10. Got: {:?}",
            diags
        );
    }

    // FP: do on continuation line of multi-line ask() call
    // entry[:phone] = ask("Phone?  ",
    //                     lambda { ... }) do |q|
    //   q.validate = ...
    // end   <-- end at col 2, aligns with entry[:phone] at col 2
    #[test]
    fn fp_do_on_continuation_line_ask() {
        let source = b"  entry[:phone] = ask(\"Phone?  \",\n                      lambda { |p| p.to_s }) do |q|\n    q.validate = true\n  end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "FP: end at col 2 aligns with entry[:phone] at col 2. Got: {:?}",
            diags
        );
    }

    // FP: do on continuation line with multi-line args (openstreetmap pattern)
    // lists << tag.ul(:class => [...]) do
    //   ...
    // end   <-- end at col 6, aligns with do-line indent or lists at col 6
    #[test]
    fn fp_multiline_args_tag_ul() {
        let source = b"      lists << tag.ul(:class => [\n                        \"pagination\",\n                      ]) do\n        items.each do |page|\n          concat page\n        end\n      end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "FP: end at col 6 aligns with lists at col 6. Got: {:?}",
            diags
        );
    }

    // FP: .select do on continuation line of chained call (openproject pattern)
    // custom_fields
    //   .select do |cf|
    //     cf.something
    // end   <-- end at col 6, aligns with custom_fields indent
    #[test]
    fn fp_select_do_continuation_chain() {
        let source = b"      RequestStore.store[key] = custom_fields\n                                   .select do |cf|\n        cf.available?\n      end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "FP: end at col 6 aligns with do-line indent or expression. Got: {:?}",
            diags
        );
    }

    // FN: end misaligned in %w[].each (floere pattern)
    #[test]
    fn fn_end_misaligned_each_block() {
        let source = b"%w[cpu object].each do |thing|\n  profile thing do\n    10_000.times { method }\n  end\n end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "FN: end at col 1 misaligned with %w at col 0. Got: {:?}",
            diags
        );
    }

    #[test]
    fn fn_exact_companybook_or_wrapper() {
        let source = b"def changed?\n  to_be_destroyed.any? || proxy_target.any? do |record|\n                                    record.new_record? || record.destroyed? || record.changed?\n                                  end\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.iter().any(|diag| diag.location.line == 4),
            "Exact CompanyBook snippet should flag the misaligned end on line 4. Got: {:?}",
            diags
        );
    }

    #[test]
    fn fn_exact_strong_password_hash_each() {
        let source = b"describe '.calculate_bonus_bits_for' do\n\t    {\n\t      'Ab$9' => 4,\n\t      'blah blah blah blah' => 1\n\t    }.each do |password, bonus_bits|\n\t      it \"returns #{bonus_bits} for '#{password}'\" do\n\t        expect(NistBonusBits.calculate_bonus_bits_for(password)).to eq(bonus_bits)\n        end\n      end\n    end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.iter().any(|diag| diag.location.line == 9),
            "Exact strong_password snippet should flag the outer end on line 9. Got: {:?}",
            diags
        );
    }
}
