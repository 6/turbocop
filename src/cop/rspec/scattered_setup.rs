use ruby_prism::Visit;

use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/ScatteredSetup flags multiple `before`/`after` hooks of the same type,
/// scope, and metadata in the same example group.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// FP=2: hooks inside `RSpec.shared_context` were incorrectly treated as part
/// of the surrounding example-group scope when the shared-context call used an
/// explicit `RSpec.` receiver. Fixed by treating receiverful `RSpec.shared_*`
/// calls as scope boundaries in the recursive collector.
///
/// FN=0: no missing detections were reported for this cop in corpus data.
///
/// ## Investigation notes
/// - **FP root cause (28 FPs):** The cop incorrectly triggered inside `shared_context`
///   and `shared_examples` blocks. RuboCop's `example_group?` matcher only matches
///   `ExampleGroups.all`, which excludes `SharedGroups.all`. Fixed by excluding shared
///   groups from triggering.
/// - **FN root cause (408 FNs):** The cop only checked direct statement children.
///   RuboCop uses `find_all_in_scope` which recursively searches within the example
///   group, stopping at scope changes (nested example groups, shared groups, includes)
///   and example blocks. Hooks inside `if` blocks, `path` blocks, etc. were missed.
///   Fixed by implementing recursive Visit-based search.
/// - **Hook name grouping:** RuboCop groups by exact hook name (`before`, `prepend_before`,
///   `append_before` are separate groups). Previously we grouped them together incorrectly.
/// - **Scope normalization:** `:each` and `:example` are equivalent (→ `:each`); `:all` and
///   `:context` are equivalent (→ `:context`). No scope arg defaults to `:each`. Hash-type
///   first arg also defaults to `:each`.
/// - **Metadata:** Additional symbol args (`:special_case`) and keyword hash args
///   (`special_case: true`) form the metadata. Symbol `:foo` is equivalent to
///   `foo: true` in RuboCop's normalization.
/// - **Excluded hooks:** `around` hooks are not checked (RuboCop explicitly skips them).
/// - **Class method hooks:** Hooks inside `defs` or `def` inside `class << self` are
///   skipped, matching RuboCop's `inside_class_method?` check.
/// - **Knowable scope:** Hooks whose first arg is not nil, sym, or hash are skipped
///   (e.g., `before(variable)` where scope can't be statically determined).
pub struct ScatteredSetup;

/// Scope keywords recognized by RSpec.
const SCOPE_KEYWORDS: &[&[u8]] = &[b"each", b"example", b"all", b"context", b"suite"];

fn is_scope_keyword(name: &[u8]) -> bool {
    SCOPE_KEYWORDS.contains(&name)
}

/// Normalize scope: :each/:example/no-arg → "each", :all/:context → "context", :suite → "suite".
fn normalize_scope(scope: &[u8]) -> &'static [u8] {
    match scope {
        b"each" | b"example" => b"each",
        b"all" | b"context" => b"context",
        b"suite" => b"suite",
        _ => b"each",
    }
}

/// Check if a hook call has a knowable scope (first arg is nil, sym, or hash).
/// If the first arg is something else (e.g., a variable), we can't determine the scope.
fn has_knowable_scope(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = call.arguments() {
        let args_list = args.arguments();
        if args_list.is_empty() {
            return true;
        }
        let first = args_list.iter().next().unwrap();
        // Accept: no args, symbol, keyword hash
        first.as_symbol_node().is_some() || first.as_keyword_hash_node().is_some()
    } else {
        true // no args = knowable (defaults to :each)
    }
}

/// Build a grouping key for a hook call: hook_name + normalized scope + sorted metadata.
/// Metadata consists of additional symbol args (beyond the scope) and keyword args.
/// Symbol `:foo` is normalized to `foo:true`, matching RuboCop's behavior.
#[allow(clippy::needless_borrow)]
fn build_hook_key(
    hook_name: &[u8],
    call: &ruby_prism::CallNode<'_>,
    source: &SourceFile,
) -> Vec<u8> {
    let mut scope: &[u8] = b"each";
    let mut metadata_parts: Vec<Vec<u8>> = Vec::new();

    if let Some(args) = call.arguments() {
        let mut found_scope = false;
        for arg in args.arguments().iter() {
            if let Some(sym) = arg.as_symbol_node() {
                let unescaped = sym.unescaped();
                // First symbol that looks like a scope keyword is the scope
                if !found_scope && is_scope_keyword(&unescaped) {
                    scope = normalize_scope(&unescaped);
                    found_scope = true;
                } else {
                    // Additional symbol args are metadata, equivalent to `name: true`
                    let mut part = unescaped.to_vec();
                    part.extend_from_slice(b":true");
                    metadata_parts.push(part);
                }
            } else if let Some(kw_hash) = arg.as_keyword_hash_node() {
                // If the first arg is a hash (no scope symbol), scope defaults to :each
                // Keyword args like `special_case: true`
                for element in kw_hash.elements().iter() {
                    if let Some(assoc) = element.as_assoc_node() {
                        let key_src = source.byte_slice(
                            assoc.key().location().start_offset(),
                            assoc.key().location().end_offset(),
                            "",
                        );
                        let val_src = source.byte_slice(
                            assoc.value().location().start_offset(),
                            assoc.value().location().end_offset(),
                            "",
                        );
                        // Strip trailing colon from symbol key if present (Prism includes it)
                        let key_bytes = key_src.as_bytes();
                        let key_clean = if key_bytes.ends_with(b":") {
                            &key_bytes[..key_bytes.len() - 1]
                        } else {
                            key_bytes
                        };
                        let mut part = key_clean.to_vec();
                        part.push(b':');
                        part.extend_from_slice(val_src.as_bytes());
                        metadata_parts.push(part);
                    }
                }
            }
        }
    }

    metadata_parts.sort();

    let mut key = Vec::new();
    key.extend_from_slice(hook_name);
    key.push(b'|');
    key.extend_from_slice(scope);
    for part in &metadata_parts {
        key.push(b'|');
        key.extend_from_slice(part);
    }
    key
}

/// Collected hook info: grouping key, line, column.
struct HookInfo {
    key: Vec<u8>,
    hook_name: Vec<u8>,
    line: usize,
    column: usize,
}

/// Visitor that recursively collects hooks within an example group scope.
/// Stops recursion at scope changes (nested example groups, shared groups, includes)
/// and example blocks, matching RuboCop's `find_all_in_scope` behavior.
struct HookCollector<'a> {
    source: &'a SourceFile,
    hooks: Vec<HookInfo>,
    /// Track whether we're inside a class method (defs or def inside class << self)
    inside_class_method: bool,
    /// Track whether we're inside class << self
    inside_sclass: bool,
}

impl<'a> HookCollector<'a> {
    fn new(source: &'a SourceFile) -> Self {
        Self {
            source,
            hooks: Vec::new(),
            inside_class_method: false,
            inside_sclass: false,
        }
    }

    /// Check if a call node is a scope change (example group, shared group, or include).
    fn is_scope_change(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name().as_slice();
        if call.receiver().is_some() {
            // Receiverful RSpec group/shared-group declarations are scope changes.
            if let Some(recv) = call.receiver() {
                return util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                    && (is_rspec_example_group(name) || is_rspec_shared_group(name));
            }
            return false;
        }
        // Example groups and shared groups are scope changes
        if is_rspec_example_group(name) || is_rspec_shared_group(name) {
            return true;
        }
        // Include methods are scope changes
        if name == b"include_examples"
            || name == b"it_behaves_like"
            || name == b"it_should_behave_like"
            || name == b"include_context"
        {
            return true;
        }
        false
    }

    /// Check if a call node is an example (it, specify, etc.).
    fn is_example(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        call.receiver().is_none() && is_rspec_example(call.name().as_slice())
    }

    /// Check if a call is a hook we care about (before/after variants, NOT around).
    fn is_relevant_hook(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        if call.receiver().is_some() {
            return false;
        }
        let name = call.name().as_slice();
        matches!(
            name,
            b"before"
                | b"prepend_before"
                | b"append_before"
                | b"after"
                | b"prepend_after"
                | b"append_after"
        )
    }
}

impl<'a, 'pr> Visit<'pr> for HookCollector<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // If this is a scope change or example with a block, don't recurse into it
        if (self.is_scope_change(node) || self.is_example(node)) && node.block().is_some() {
            return;
        }

        // Check if this is a relevant hook
        if self.is_relevant_hook(node) && !self.inside_class_method && node.block().is_some() {
            if has_knowable_scope(node) {
                let hook_name = node.name().as_slice();
                let key = build_hook_key(hook_name, node, self.source);
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.hooks.push(HookInfo {
                    key,
                    hook_name: hook_name.to_vec(),
                    line,
                    column,
                });
            }
            // Don't recurse into hook body
            return;
        }

        // Default traversal for other nodes
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // `def self.method` (has receiver) is a class method
        // `def method` inside `class << self` is also a class method
        let prev = self.inside_class_method;
        if node.receiver().is_some() || self.inside_sclass {
            self.inside_class_method = true;
        }
        ruby_prism::visit_def_node(self, node);
        self.inside_class_method = prev;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let prev_sclass = self.inside_sclass;
        self.inside_sclass = true;
        ruby_prism::visit_singleton_class_node(self, node);
        self.inside_sclass = prev_sclass;
    }
}

impl Cop for ScatteredSetup {
    fn name(&self) -> &'static str {
        "RSpec/ScatteredSetup"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Only trigger for example groups, NOT shared groups
        // RuboCop's example_group? matcher uses ExampleGroups.all which excludes SharedGroups
        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec") && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name) && !is_rspec_shared_group(method_name)
        };

        if !is_example_group {
            return;
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        // Recursively collect hooks within this scope
        let mut collector = HookCollector::new(source);
        collector.visit(&body);

        // Group hooks by key and flag duplicates
        let mut groups: std::collections::HashMap<Vec<u8>, Vec<&HookInfo>> =
            std::collections::HashMap::new();
        for hook in &collector.hooks {
            groups.entry(hook.key.clone()).or_default().push(hook);
        }

        for hooks in groups.values() {
            if hooks.len() > 1 {
                for hook in hooks {
                    let other_lines: Vec<String> = hooks
                        .iter()
                        .filter(|h| h.line != hook.line)
                        .map(|h| h.line.to_string())
                        .collect();
                    let also = if other_lines.len() == 1 {
                        format!("line {}", other_lines[0])
                    } else {
                        format!("lines {}", other_lines.join(", "))
                    };
                    let hook_display = String::from_utf8_lossy(&hook.hook_name).into_owned();
                    diagnostics.push(self.diagnostic(
                        source,
                        hook.line,
                        hook.column,
                        format!(
                            "Do not define multiple `{hook_display}` hooks in the same example group (also defined on {also})."
                        ),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ScatteredSetup, "cops/rspec/scattered_setup");
}
