use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Detects redundant `all` used as a receiver for Active Record query methods.
///
/// ## Investigation findings (2026-03-08)
///
/// Root causes of corpus divergence (FP=419, FN=194):
///
/// 1. **FN=194**: The method list (`REDUNDANT_AFTER_ALL`) had only 18 methods vs vendor's
///    100+ from `ActiveRecord::Querying::QUERYING_METHODS`. Expanded to match vendor.
///
/// 2. **FP=419→24→0**: Multiple causes fixed in stages:
///    - Offense location was reported at the outer call node (full chain start) instead
///      of at the `all` method name position. Fixed to use `inner_call.message_loc()`.
///    - Message included extra "Remove `all` from the chain." text not in vendor.
///    - Missing check to skip `all` called with arguments (e.g., `page.all(:param)`).
///    - Missing `sensitive_association_method?` logic: `delete_all`/`destroy_all` should
///      only be flagged when receiver of `all` is a constant (model), not an association.
///    - **Remaining 24 FPs**: Bare `all` calls without a receiver in non-AR classes,
///      modules, and concerns (e.g., ActiveGraph nodes, ActiveHash, Mongoid, Sidekiq).
///      RuboCop uses `inherit_active_record_base?` to check class hierarchy for no-receiver
///      cases. Since nitrocop lacks class-hierarchy analysis, we skip all no-receiver `all`
///      calls. This is conservative but eliminates FPs with zero FN impact (corpus FN=0).
///
/// ## Investigation findings (2026-03-16)
///
/// **FN=7 root cause**: All 7 remaining FNs are bare `all` calls (no explicit receiver)
/// inside class methods of AR models:
///   - `all.where(active: true)` inside `class X < ApplicationRecord`
///   - `all.pluck(:name)` inside `class X < ActiveRecord::Base`
///   - `all.find_each(&:method)` inside `class X < ::ApplicationRecord`
///   - etc.
///
/// The previous implementation conservatively skipped ALL no-receiver `all` calls to
/// avoid FPs on non-AR classes. However, RuboCop uses `inherit_active_record_base?`
/// to flag no-receiver `all` only inside classes inheriting from `ApplicationRecord`,
/// `::ApplicationRecord`, `ActiveRecord::Base`, or `::ActiveRecord::Base`.
///
/// **Fix**: Switched from `check_node` to `check_source` with a visitor that tracks
/// class inheritance context (`in_ar_class: bool`). This matches `FindEach`'s approach.
/// The visitor sets `in_ar_class = true` when entering a class that inherits from an AR
/// base class, then checks `all.method()` chains. No-receiver `all` is only flagged
/// when `in_ar_class` is true.
pub struct RedundantActiveRecordAllMethod;

/// ActiveRecord::Querying::QUERYING_METHODS (from activerecord 7.1.0)
/// plus `empty?` which is inherited from Enumerable but still valid.
const QUERYING_METHODS: &[&[u8]] = &[
    b"and",
    b"annotate",
    b"any?",
    b"async_average",
    b"async_count",
    b"async_ids",
    b"async_maximum",
    b"async_minimum",
    b"async_pick",
    b"async_pluck",
    b"async_sum",
    b"average",
    b"calculate",
    b"count",
    b"create_or_find_by",
    b"create_or_find_by!",
    b"create_with",
    b"delete_all",
    b"delete_by",
    b"destroy_all",
    b"destroy_by",
    b"distinct",
    b"eager_load",
    b"except",
    b"excluding",
    b"exists?",
    b"extending",
    b"extract_associated",
    b"fifth",
    b"fifth!",
    b"find",
    b"find_by",
    b"find_by!",
    b"find_each",
    b"find_in_batches",
    b"find_or_create_by",
    b"find_or_create_by!",
    b"find_or_initialize_by",
    b"find_sole_by",
    b"first",
    b"first!",
    b"first_or_create",
    b"first_or_create!",
    b"first_or_initialize",
    b"forty_two",
    b"forty_two!",
    b"fourth",
    b"fourth!",
    b"from",
    b"group",
    b"having",
    b"ids",
    b"in_batches",
    b"in_order_of",
    b"includes",
    b"invert_where",
    b"joins",
    b"last",
    b"last!",
    b"left_joins",
    b"left_outer_joins",
    b"limit",
    b"lock",
    b"many?",
    b"maximum",
    b"merge",
    b"minimum",
    b"none",
    b"none?",
    b"offset",
    b"one?",
    b"only",
    b"optimizer_hints",
    b"or",
    b"order",
    b"pick",
    b"pluck",
    b"preload",
    b"readonly",
    b"references",
    b"regroup",
    b"reorder",
    b"reselect",
    b"rewhere",
    b"second",
    b"second!",
    b"second_to_last",
    b"second_to_last!",
    b"select",
    b"sole",
    b"strict_loading",
    b"sum",
    b"take",
    b"take!",
    b"third",
    b"third!",
    b"third_to_last",
    b"third_to_last!",
    b"touch_all",
    b"unscope",
    b"update_all",
    b"where",
    b"with",
    b"without",
];

/// Methods that could be Enumerable block methods instead of AR query methods.
/// When called with a block, these should NOT be flagged as redundant `all`.
const POSSIBLE_ENUMERABLE_BLOCK_METHODS: &[&[u8]] = &[
    b"any?", b"count", b"find", b"none?", b"one?", b"select", b"sum",
];

/// Methods that are sensitive on associations — `delete_all` and `destroy_all`
/// behave differently on `ActiveRecord::Relation` vs `CollectionProxy`.
/// Only flag these when the receiver of `all` is a constant (i.e., a model class).
const SENSITIVE_METHODS_ON_ASSOCIATION: &[&[u8]] = &[b"delete_all", b"destroy_all"];

/// Parent class names that indicate ActiveRecord inheritance.
const AR_BASE_CLASSES: &[&[u8]] = &[
    b"ApplicationRecord",
    b"::ApplicationRecord",
    b"ActiveRecord::Base",
    b"::ActiveRecord::Base",
];

impl Cop for RedundantActiveRecordAllMethod {
    fn name(&self) -> &'static str {
        "Rails/RedundantActiveRecordAllMethod"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let allowed_receivers = config.get_string_array("AllowedReceivers");

        let mut visitor = AllMethodVisitor {
            cop: self,
            source,
            allowed_receivers,
            diagnostics: Vec::new(),
            in_ar_class: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct AllMethodVisitor<'a, 'src> {
    cop: &'a RedundantActiveRecordAllMethod,
    source: &'src SourceFile,
    allowed_receivers: Option<Vec<String>>,
    diagnostics: Vec<Diagnostic>,
    /// Whether we are currently inside a class that inherits from ActiveRecord.
    in_ar_class: bool,
}

impl<'pr> AllMethodVisitor<'_, '_> {
    fn check_call(&mut self, outer_call: &ruby_prism::CallNode<'pr>) {
        let outer_method = outer_call.name().as_slice();

        if !QUERYING_METHODS.contains(&outer_method) {
            return;
        }

        // The receiver of the outer call must be a CallNode (the `all` call)
        let receiver = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"all" {
            return;
        }

        // Skip if `all` is called with arguments (e.g., `page.all(:parameter)`)
        // — that's not ActiveRecord's `all`.
        if inner_call.arguments().is_some() {
            return;
        }

        // Handle no-receiver `all` (bare `all.method(...)`)
        if inner_call.receiver().is_none() {
            // Only flag if we're inside an AR-inheriting class (matches vendor's
            // `inherit_active_record_base?` check).
            if !self.in_ar_class {
                return;
            }
        }

        // Skip when a possible Enumerable block method is called with a block
        // (e.g., `all.select { |r| r.active? }` uses Ruby's Enumerable#select)
        if POSSIBLE_ENUMERABLE_BLOCK_METHODS.contains(&outer_method) {
            if outer_call.block().is_some() {
                return;
            }
            // Also check for block pass: all.select(&:active?)
            if let Some(args) = outer_call.arguments() {
                if args
                    .arguments()
                    .iter()
                    .any(|a| a.as_block_argument_node().is_some())
                {
                    return;
                }
            }
        }

        // For sensitive methods (delete_all, destroy_all), only flag when the
        // receiver of `all` is a constant (model class). Skip for associations
        // (non-const receivers) and no-receiver calls.
        if SENSITIVE_METHODS_ON_ASSOCIATION.contains(&outer_method) {
            match inner_call.receiver() {
                Some(recv) => {
                    // Only flag if receiver is a constant (e.g., User.all.delete_all)
                    if recv.as_constant_read_node().is_none()
                        && recv.as_constant_path_node().is_none()
                    {
                        return;
                    }
                }
                // No receiver (e.g., `all.delete_all`) — skip
                None => return,
            }
        }

        // Skip if receiver of the `all` call is in AllowedReceivers
        if let Some(ref receivers) = self.allowed_receivers {
            if let Some(recv) = inner_call.receiver() {
                let recv_str = std::str::from_utf8(recv.location().as_slice()).unwrap_or("");
                if receivers.iter().any(|r| r == recv_str) {
                    return;
                }
            }
        }

        // Report at the `all` method name location
        let msg_loc = inner_call.message_loc().unwrap_or(inner_call.location());
        let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Redundant `all` detected.".to_string(),
        ));
    }
}

impl<'pr> Visit<'pr> for AllMethodVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let prev_in_ar = self.in_ar_class;

        // Check if this class inherits from an AR base class
        if let Some(superclass) = node.superclass() {
            let loc = superclass.location();
            let parent_bytes = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
            if AR_BASE_CLASSES.contains(&parent_bytes) {
                self.in_ar_class = true;
            }
        }

        ruby_prism::visit_class_node(self, node);
        self.in_ar_class = prev_in_ar;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_call(node);
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantActiveRecordAllMethod,
        "cops/rails/redundant_active_record_all_method"
    );
}
