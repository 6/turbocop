use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for duplicated instance (or singleton) method definitions.
///
/// Tracks `def`, `defs` (self.method), `alias`, `alias_method`, `attr_reader`,
/// `attr_writer`, `attr_accessor`, `attr`, `def_delegator`, `def_instance_delegator`,
/// `def_delegators`, and `def_instance_delegators`.
///
/// ## Investigation history
///
/// ### Round 1 (initial implementation)
/// Root causes of FN (1,203):
/// - Only checked direct `def` children of class/module StatementsNode
/// - Missed `private def`, `protected def` (CallNode wrapping DefNode)
/// - Missed `alias_method`, `attr_*`, `def_delegator*` call patterns
/// - Missed top-level definitions (Object scope)
/// - Missed reopened class/module blocks (separate `class A...end` blocks)
/// - Missed nested method scoping (method_key with ancestor def name)
/// - Missed `class << self` / `class << expr` singleton class patterns
/// - Wrong message format (was "Duplicated method definition." instead of RuboCop format)
///
/// Root causes of FP (47):
/// - Did not skip definitions inside `if`/`unless`/`case` ancestors
/// - Did not handle rescue/ensure scope reset
///
/// ### Round 2 (FP=16, FN=40)
/// Root causes of FN:
/// - `def ConstName.method` where ConstName is an outer scope (not innermost). The old
///   `scope_matches_const` only checked innermost scope. Fixed by implementing
///   `lookup_constant` that traverses the full scope stack, matching RuboCop behavior.
///
/// Root causes of FP:
/// - rescue/ensure handling used per-block scope (stack of Vecs) instead of global
///   per-type scope like RuboCop. RuboCop uses `@scopes[:rescue]` and `@scopes[:ensure]`
///   as global sets — first redefinition across ALL rescue blocks is forgiven, second is
///   offense. Fixed by replacing `rescue_ensure_stack: Vec<Vec<String>>` with global
///   `rescue_forgiven` and `ensure_forgiven` sets plus a type stack.
///
/// ### Round 3 (FP=17, FN=37)
/// Root causes of FP:
/// - `alias_method`, `attr_*`, `def_delegator*`, `def_delegators` calls with explicit
///   receiver (e.g., `self.alias_method`, `self.attr_reader`, `doc.attr('content')`) were
///   being processed. RuboCop uses `(send nil? ...)` which requires no receiver. Fixed by
///   adding a `node.receiver().is_some()` early return in `process_call`.
/// - `def_delegators` / `def_delegator` with a constant as first arg (e.g.,
///   `def_delegators SomeModule, :run, :stop`) was registering methods. RuboCop's pattern
///   requires `{sym str}` as first arg. Fixed by checking `extract_symbol_or_string` on
///   `args[0]`.
/// - `Struct.new do ... end` was treated as scope-creating (like Class.new/Module.new).
///   RuboCop's `defined_module0` NodePattern only matches `Class` and `Module` in casgn
///   context, not `Struct`. Defs inside Struct.new blocks have parent_module_name return
///   nil (block is not recognized), so they are ignored. Fixed by removing Struct from
///   scope_creating_call_name and visit_constant_write_node.
/// - `module_eval` blocks were treated as scope-creating like class_eval. RuboCop's
///   `parent_module_name_for_block` only checks `ancestor.method?(:class_eval)`, not
///   module_eval. Fixed by removing module_eval from scope_creating_call_name.
/// - Implicit `class_eval` (no receiver) was pushing a new scope entry, causing
///   double-nesting (e.g., `A::A` inside `module A`). RuboCop's class_eval without
///   receiver returns nil from parent_module_name_for_block, making it transparent.
///   Fixed by visiting the block body without scope changes for implicit class_eval.
///
/// Root causes of FN:
/// - `case`/`case_match` nodes were suppressing duplicate detection (treated like `if`).
///   RuboCop's `node.each_ancestor.any?(&:if_type?)` only matches `if`/`unless` nodes,
///   NOT `case`/`when`. Fixed by removing `visit_case_node` and `visit_case_match_node`
///   overrides that incremented `if_depth`.
///
/// ### Round 5 (FP=1, FN=26)
/// Root causes of FP:
/// - `alias_method "foo", "bar"` with string args was treated the same as
///   `alias_method :foo, :bar` with symbol args. RuboCop's `alias_method?` pattern
///   is `(send nil? :alias_method (sym $_name) (sym $_original_name))` — it only
///   matches symbol arguments. Fixed by using `extract_symbol_only` instead of
///   `extract_symbol_or_string` in `process_alias_method`.
///
/// Root causes of FN:
/// - `delegate :method, to: :target` (ActiveSupport) was not tracked. Many corpus
///   repos have `ActiveSupportExtensionsEnabled: true` via rubocop-rails. Fixed by
///   implementing `process_delegate` that reads the `ActiveSupportExtensionsEnabled`
///   config flag, matching RuboCop's `delegate_method?` pattern. Handles `prefix: true`
///   and `prefix: :name` options.
/// - `class << A::B` (ConstantPathNode) in singleton class expression was not handled.
///   Only `ConstantReadNode` was matched. Fixed by adding `as_constant_path_node()`
///   check in `visit_singleton_class_node`.
///
/// ### Round 6 (FP=3, FN=26)
/// Root causes of FP:
/// - `class << Multiton::ClassMethods` with nested `class InstanceMutex` was resolving
///   to the same scope as `module Multiton > module ClassMethods > class << self >
///   class InstanceMutex`. Both produced key `Multiton::ClassMethods::InstanceMutex#method`.
///   In RuboCop, `parent_module_name` returns nil for defs inside non-self sclass, and
///   `found_sclass_method` only handles send-type receivers (not const/const_path).
///   This means ALL methods inside `class << SomeConst` are invisible to RuboCop's
///   duplicate detection. Fixed by treating non-self sclass bodies as plain blocks
///   (incrementing `plain_block_depth`), matching RuboCop's behavior.
///
/// ### Round 7 (FP=0, FN=37)
/// Root causes of FN:
/// - `class << ConstName` (e.g., `class << Multiton`, `class << SymPlane`,
///   `class << Multiton::ClassMethods`) was treated as invisible
///   (`plain_block_depth += 1`). Round 6 made this change to avoid FP from
///   nested classes inside sclass-const vs sclass-self contexts. However, the
///   Round 6 analysis was incorrect about RuboCop behavior: `parent_module_name`
///   DOES return `#<Class:ConstName>` for defs inside `class << ConstName`, and
///   the humanization regex produces `ConstName.method` — so duplicates ARE detected.
///   Fixed by pushing `#<Class:ConstName>` as the scope name and applying
///   `humanize_scope()` in `qualified_method_name()`, which converts
///   `#<Class:X>` → `X.` matching RuboCop's regex. Nested classes produce
///   distinct keys because `#<Class:X>::Nested` humanizes to `X.Nested` while
///   `X::#<Class:X>::Nested` humanizes to `X.::Nested`. (11 FN fixed, 0 FP)
///
/// ### Round 8 (FP=0, FN=26)
/// Root causes of FN:
/// - `ActiveSupportExtensionsEnabled` was not being injected from AllCops config
///   into `Lint/DuplicateMethods` cop config. The config injection code in
///   `src/config/mod.rs` only listed `Style/CollectionQuerying` and
///   `Style/RedundantFilterChain`, not `Lint/DuplicateMethods`. This meant
///   `delegate` tracking was disabled for all corpus repos despite rubocop-rails
///   setting `AllCops.ActiveSupportExtensionsEnabled: true`. Fixed by adding
///   `Lint/DuplicateMethods` to the injection list. (20+ delegate-related FN fixed)
/// - `def ConstName.method` inside DSL blocks (plain_block_depth > 0) was being
///   skipped. RuboCop's `on_defs` handler for const-receiver defs uses
///   `check_const_receiver` which resolves via AST ancestors,
///   not `parent_module_name`. It works inside blocks as long as the constant is
///   in scope. Fixed by only applying the `plain_block_depth > 0` early return
///   to instance methods and `def self.method`, not `def ConstName.method`.
///
/// ### Round 9 (FP=0, FN=4 standard; FP=6, FN=53 extended)
/// Root causes of FN:
/// - `class << call_expr` (e.g., `class << Object.new`, `class << @reflex.controller.response`)
///   was treated as invisible (`plain_block_depth += 1`). RuboCop's `found_sclass_method`
///   handles this case: when `parent_module_name` returns nil and the sclass expression is
///   a `send_type?` (method call), it tracks methods as `receiver_method_name.method_name`.
///   Fixed by checking if the sclass expression is a CallNode, extracting its method name,
///   and pushing it as a singleton scope. Methods inside are now tracked (e.g., `def body`
///   inside `class << response` becomes `response.body`).
///
///
/// ### Round 10 (FP=0, FN=4 standard)
/// Root causes of FN:
/// - `def ConstName.method` where constant is NOT in the scope stack (no enclosing
///   class/module/casgn ancestor defines it). Previously skipped when `lookup_constant`
///   returned None. However, RuboCop's `lookup_constant` has a quirk: `each_ancestor`
///   with a block returns the receiver (the node itself) when the block never executes
///   (no matching ancestors). So `qualified` becomes the node's AST dump. Two identical
///   defs produce the same AST dump key and are detected; different bodies produce
///   different keys. Fixed by using the def's source text as the dedup key when
///   `lookup_constant` returns None, matching this behavior. (FN1: seedbank, FN4: vcr)
/// - `def meth` inside `Class.new { }` block within a `class << call_expr` expression
///   was not visited because only the sclass body was visited, not the expression.
///   RuboCop catches these via `found_sclass_method` which uses ancestor traversal
///   to find the enclosing sclass. Fixed by scanning the sclass expression for DefNode
///   descendants and registering them in the sclass scope. (FN2: pry)
/// - Reopened `class << call_expr` blocks (e.g., two separate
///   `class << @reflex.controller.response` blocks) already worked correctly since
///   both push the same scope name and share the global definitions map. (FN3:
///   stimulus_reflex was already handled)
pub struct DuplicateMethods;

impl Cop for DuplicateMethods {
    fn name(&self) -> &'static str {
        "Lint/DuplicateMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let active_support = config.get_bool("ActiveSupportExtensionsEnabled", false);
        let mut visitor = DupMethodVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            definitions: HashMap::new(),
            scope_stack: Vec::new(),
            def_stack: Vec::new(),
            if_depth: 0,
            plain_block_depth: 0,
            in_rescue_or_ensure: false,
            rescue_forgiven: Vec::new(),
            ensure_forgiven: Vec::new(),
            rescue_ensure_type_stack: Vec::new(),
            active_support_extensions: active_support,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Stored definition location for a method.
#[derive(Clone)]
struct DefLocation {
    line: usize,
}

struct DupMethodVisitor<'a, 'src> {
    cop: &'a DuplicateMethods,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Global definitions map: qualified method key -> first definition location
    definitions: HashMap<String, DefLocation>,
    /// Stack of scope names for building qualified method names.
    /// Each entry tracks the scope name and whether we are inside a singleton class.
    scope_stack: Vec<ScopeEntry>,
    /// Stack of enclosing def method names (for method_key scoping of nested defs)
    def_stack: Vec<String>,
    /// Depth inside if/unless nodes — skip definitions when > 0
    if_depth: usize,
    /// Depth inside non-scope blocks (DSL blocks, describe blocks, etc.)
    /// Methods inside these are ignored per RuboCop (parent_module_name returns nil).
    plain_block_depth: usize,
    /// Whether we are currently inside a rescue or ensure node (any depth).
    /// Matches RuboCop's `node.each_ancestor(:rescue, :ensure).first&.type`.
    in_rescue_or_ensure: bool,
    /// Global per-type scope for rescue blocks. Keys forgiven in any rescue block
    /// are tracked here. Matches RuboCop's `@scopes[:rescue]`.
    rescue_forgiven: Vec<String>,
    /// Global per-type scope for ensure blocks. Keys forgiven in any ensure block
    /// are tracked here. Matches RuboCop's `@scopes[:ensure]`.
    ensure_forgiven: Vec<String>,
    /// Which scope type the current rescue/ensure ancestors belong to.
    /// Used to decide which forgiven set to check.
    rescue_ensure_type_stack: Vec<RescueEnsureType>,
    /// Whether ActiveSupport extensions are enabled (for `delegate` tracking).
    active_support_extensions: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RescueEnsureType {
    Rescue,
    Ensure,
}

#[derive(Clone)]
struct ScopeEntry {
    name: String,
    is_singleton: bool,
}

impl DupMethodVisitor<'_, '_> {
    /// Build the qualified method name like RuboCop's `found_instance_method`.
    /// For instance methods: `ClassName#method_name`
    /// For singleton methods: `ClassName.method_name`
    /// For top-level: `Object#method_name`
    ///
    /// When the scope contains `#<Class:ConstName>` entries (from `class << ConstName`),
    /// applies RuboCop's humanization regex to produce readable names like `ConstName.method`.
    fn qualified_method_name(&self, method_name: &str, is_singleton: bool) -> String {
        let scope = self.current_scope_name();
        let humanized = humanize_scope(&scope);

        if humanized.ends_with('.') {
            // Scope already ends with `.` from humanization (e.g., `ConstName.`)
            // so the method is implicitly a singleton method
            format!("{humanized}{method_name}")
        } else {
            let separator = if is_singleton { "." } else { "#" };
            format!("{humanized}{separator}{method_name}")
        }
    }

    /// Get the current scope name from the scope stack.
    fn current_scope_name(&self) -> String {
        if self.scope_stack.is_empty() {
            return "Object".to_string();
        }

        let mut parts = Vec::new();
        for entry in &self.scope_stack {
            parts.push(entry.name.as_str());
        }
        parts.join("::")
    }

    /// Build the method key that includes ancestor def name for nested methods.
    /// This matches RuboCop's `method_key` method.
    fn method_key(&self, qualified_name: &str) -> String {
        if let Some(ancestor_def) = self.def_stack.last() {
            format!("{ancestor_def}.{qualified_name}")
        } else {
            qualified_name.to_string()
        }
    }

    /// Record a found method definition and check for duplicates.
    fn found_method(
        &mut self,
        method_name: &str,
        is_singleton: bool,
        def_line: usize,
        offense_offset: usize,
    ) {
        let qualified = self.qualified_method_name(method_name, is_singleton);
        let key = self.method_key(&qualified);

        // Handle rescue/ensure scope: first occurrence of a method key inside
        // ANY rescue (or ensure) body is allowed to "redefine" — it's a different
        // execution path. RuboCop uses @scopes[:rescue] / @scopes[:ensure] as
        // global per-type sets (not per-block). So the first redefinition across
        // ALL rescue blocks is forgiven, but the second is an offense.
        if self.in_rescue_or_ensure && self.definitions.contains_key(&key) {
            if let Some(&scope_type) = self.rescue_ensure_type_stack.last() {
                let forgiven = match scope_type {
                    RescueEnsureType::Rescue => &self.rescue_forgiven,
                    RescueEnsureType::Ensure => &self.ensure_forgiven,
                };
                if !forgiven.contains(&key) {
                    // First time this key is redefined in this scope type — forgive it
                    self.definitions
                        .insert(key.clone(), DefLocation { line: def_line });
                    match scope_type {
                        RescueEnsureType::Rescue => self.rescue_forgiven.push(key),
                        RescueEnsureType::Ensure => self.ensure_forgiven.push(key),
                    }
                    return;
                }
            }
        }

        if let Some(first_def) = self.definitions.get(&key) {
            let first_line = first_def.line;
            let path = self.source.path_str();
            let (line, column) = self.source.offset_to_line_col(offense_offset);
            let message = format!(
                "Method `{qualified}` is defined at both {path}:{first_line} and {path}:{line}."
            );
            let diag = self.cop.diagnostic(self.source, line, column, message);
            self.diagnostics.push(diag);
        } else {
            self.definitions.insert(key, DefLocation { line: def_line });
        }
    }

    /// Process a def node (instance or singleton method).
    ///
    /// RuboCop's `on_defs` handler for `def ConstName.method` uses `check_const_receiver`
    /// which resolves the constant via `lookup_constant` and does NOT check
    /// `parent_module_name` — so it works even inside DSL blocks. In contrast,
    /// `on_def` (instance methods) and `check_self_receiver` (`def self.method`)
    /// use `parent_module_name` which returns nil inside blocks, effectively
    /// ignoring those definitions.
    ///
    /// We match this by only applying the `plain_block_depth > 0` early return
    /// for instance methods and `def self.method`, but NOT for `def ConstName.method`.
    fn process_def(&mut self, node: &ruby_prism::DefNode<'_>) {
        // if/unless always suppresses duplicate detection
        if self.if_depth > 0 {
            return;
        }

        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());

        if let Some(receiver) = node.receiver() {
            // def self.method or def ConstName.method
            if receiver.as_self_node().is_some() {
                // def self.method inside blocks: parent_module_name returns nil in RuboCop,
                // so these are not tracked. Skip when in a plain block.
                if self.plain_block_depth > 0 {
                    return;
                }
                let keyword_offset = node.def_keyword_loc().start_offset();
                self.found_method(name, true, def_line, keyword_offset);
            } else if let Some(const_read) = receiver.as_constant_read_node() {
                // def ConstName.method: RuboCop's check_const_receiver resolves the constant
                // independently of parent_module_name, so it works inside blocks too.
                let const_name = std::str::from_utf8(const_read.name().as_slice()).unwrap_or("");
                if let Some(qualified_scope) = self.lookup_constant(const_name) {
                    // Save current scope, temporarily replace with resolved scope
                    let saved_scopes = self.scope_stack.clone();
                    self.scope_stack.clear();
                    self.scope_stack.push(ScopeEntry {
                        name: qualified_scope,
                        is_singleton: true,
                    });
                    let keyword_offset = node.def_keyword_loc().start_offset();
                    self.found_method(name, true, def_line, keyword_offset);
                    self.scope_stack = saved_scopes;
                } else {
                    // Constant not in scope stack. RuboCop's lookup_constant returns the
                    // defs node itself (as a Ruby object) due to `each_ancestor` returning
                    // the receiver when the block never executes. This means the "qualified"
                    // name becomes the AST dump of the full def node. Two defs with identical
                    // AST (same receiver, params, body) produce the same key and are detected
                    // as duplicates; defs with different bodies produce different keys.
                    //
                    // We replicate this by using the source text of the def node as the
                    // dedup key, but producing a clean `ConstName.method` message format.
                    let loc = node.location();
                    let start = loc.start_offset();
                    let end_off = loc.end_offset();
                    let source_bytes = self.source.as_bytes();
                    if end_off <= source_bytes.len() {
                        let def_source =
                            std::str::from_utf8(&source_bytes[start..end_off]).unwrap_or("");
                        // Use source text as the dedup key but const_name for the message
                        let qualified = format!("{const_name}.{name}");
                        let key = self.method_key(&format!("{def_source}.{name}"));

                        if let Some(first_def) = self.definitions.get(&key) {
                            let first_line = first_def.line;
                            let path = self.source.path_str();
                            let keyword_offset = node.def_keyword_loc().start_offset();
                            let (line, column) = self.source.offset_to_line_col(keyword_offset);
                            let message = format!(
                                "Method `{qualified}` is defined at both \
                                 {path}:{first_line} and {path}:{line}."
                            );
                            let diag = self.cop.diagnostic(self.source, line, column, message);
                            self.diagnostics.push(diag);
                        } else {
                            self.definitions.insert(key, DefLocation { line: def_line });
                        }
                    }
                }
            } else {
                // def expr.method (non-const, non-self receiver) — not tracked
            }
        } else {
            // Instance method (or singleton method if inside `class << self`)
            // Inside blocks: parent_module_name returns nil, so not tracked.
            if self.plain_block_depth > 0 {
                return;
            }
            let is_singleton = self.in_singleton_scope();
            let keyword_offset = node.def_keyword_loc().start_offset();
            self.found_method(name, is_singleton, def_line, keyword_offset);
        }
    }

    /// Check if we're currently inside a singleton class (class << self).
    fn in_singleton_scope(&self) -> bool {
        self.scope_stack.last().is_some_and(|e| e.is_singleton)
    }

    /// Look up a constant name in the scope stack, matching RuboCop's `lookup_constant`.
    /// Returns the fully qualified scope name if found, or None if not found.
    fn lookup_constant(&self, const_name: &str) -> Option<String> {
        // Walk the scope stack from innermost to outermost, looking for a match.
        // Each scope entry may be a simple name ("A") or a constant path ("A::B").
        // Check each component of the name.
        for (i, entry) in self.scope_stack.iter().enumerate().rev() {
            // Check if this entry's name matches directly or as a component
            let parts: Vec<&str> = entry.name.split("::").collect();
            for part in &parts {
                if *part == const_name {
                    // Found the matching constant. Build the qualified name
                    // up to and including this scope entry.
                    let mut result_parts = Vec::new();
                    for e in &self.scope_stack[..=i] {
                        result_parts.push(e.name.as_str());
                    }
                    return Some(result_parts.join("::"));
                }
            }
        }
        None
    }

    /// Collect def nodes from a sclass expression and register them in the current scope.
    ///
    /// RuboCop's `found_sclass_method` catches defs inside the sclass expression
    /// (not just the body) because it uses `each_ancestor(:sclass_type?)` which
    /// finds the enclosing sclass even when the def is inside a block within the
    /// expression. For example:
    ///   `class << Class.new { def meth; 1; end }.new`
    /// The `def meth` is inside `Class.new { }` block in the expression, but
    /// RuboCop still tracks it as `new.meth` via the sclass ancestor.
    ///
    /// We replicate this by recursively scanning the expression for DefNode
    /// descendants (without receivers) and registering them. The current scope
    /// should already be set to the sclass scope before calling this method.
    fn collect_defs_from_sclass_expr(&mut self, node: &ruby_prism::Node<'_>) {
        // Use the Visit trait to find all DefNode descendants
        struct DefCollector<'a> {
            defs: Vec<(String, usize, usize)>, // (name, def_line, keyword_offset)
            source: &'a SourceFile,
        }
        impl<'pr> Visit<'pr> for DefCollector<'_> {
            fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
                // Only collect instance methods (no receiver) — matching RuboCop's
                // found_sclass_method which calls found_instance_method
                if node.receiver().is_none() {
                    let name = std::str::from_utf8(node.name().as_slice())
                        .unwrap_or("")
                        .to_string();
                    let (def_line, _) = self
                        .source
                        .offset_to_line_col(node.location().start_offset());
                    let keyword_offset = node.def_keyword_loc().start_offset();
                    self.defs.push((name, def_line, keyword_offset));
                }
                // Don't recurse into the def body — nested defs are separate
            }
        }

        let mut collector = DefCollector {
            defs: Vec::new(),
            source: self.source,
        };
        collector.visit(node);

        for (name, def_line, keyword_offset) in collector.defs {
            // The scope is already set to the sclass call scope (e.g., "new")
            // and is_singleton is true, so this will produce e.g., "new.meth"
            let is_singleton = self.in_singleton_scope();
            self.found_method(&name, is_singleton, def_line, keyword_offset);
        }
    }

    /// Process an alias node.
    fn process_alias(&mut self, node: &ruby_prism::AliasMethodNode<'_>) {
        if self.if_depth > 0 || self.plain_block_depth > 0 {
            return;
        }

        let new_name_node = node.new_name();
        let old_name_node = node.old_name();

        let new_sym = match new_name_node.as_symbol_node() {
            Some(s) => s,
            None => return,
        };

        // Self-alias is allowed (alias foo foo)
        if let Some(old_sym) = old_name_node.as_symbol_node() {
            if new_sym.unescaped() == old_sym.unescaped() {
                return;
            }
        }

        let name = std::str::from_utf8(new_sym.unescaped()).unwrap_or("");
        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();
        self.found_method(name, is_singleton, def_line, offset);
    }

    /// Process a call node for alias_method, attr_*, def_delegator*, etc.
    /// RuboCop only matches these as `(send nil? :method_name ...)`, meaning
    /// they must have no explicit receiver. Calls like `self.attr_reader` or
    /// `doc.attr('content')` are ignored.
    fn process_call(&mut self, node: &ruby_prism::CallNode<'_>) {
        if self.if_depth > 0 || self.plain_block_depth > 0 {
            return;
        }

        // Only match bare calls (no receiver), matching RuboCop's `(send nil? ...)` pattern
        if node.receiver().is_some() {
            return;
        }

        let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        let args = match node.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();

        match method_name {
            "alias_method" => self.process_alias_method(node, &arg_list),
            "attr_reader" => self.process_attr(node, &arg_list, true, false),
            "attr_writer" => self.process_attr(node, &arg_list, false, true),
            "attr_accessor" => self.process_attr(node, &arg_list, true, true),
            "attr" => self.process_attr_legacy(node, &arg_list),
            "def_delegator" | "def_instance_delegator" => {
                self.process_def_delegator(node, &arg_list);
            }
            "def_delegators" | "def_instance_delegators" => {
                self.process_def_delegators(node, &arg_list);
            }
            "delegate" if self.active_support_extensions => {
                self.process_delegate(node, &arg_list);
            }
            _ => {}
        }
    }

    fn process_alias_method(
        &mut self,
        node: &ruby_prism::CallNode<'_>,
        args: &[ruby_prism::Node<'_>],
    ) {
        if args.len() < 2 {
            return;
        }
        // RuboCop's alias_method? pattern is:
        //   (send nil? :alias_method (sym $_name) (sym $_original_name))
        // It only matches symbol arguments, not string arguments.
        let new_name = match extract_symbol_only(&args[0]) {
            Some(n) => n,
            None => return,
        };
        let orig_name = match extract_symbol_only(&args[1]) {
            Some(n) => n,
            None => return,
        };
        if new_name == orig_name {
            return;
        }

        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();
        self.found_method(&new_name, is_singleton, def_line, offset);
    }

    fn process_attr(
        &mut self,
        node: &ruby_prism::CallNode<'_>,
        args: &[ruby_prism::Node<'_>],
        readable: bool,
        writable: bool,
    ) {
        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();

        for arg in args {
            if let Some(name) = extract_symbol_or_string(arg) {
                if readable {
                    self.found_method(&name, is_singleton, def_line, offset);
                }
                if writable {
                    let setter = format!("{name}=");
                    self.found_method(&setter, is_singleton, def_line, offset);
                }
            }
        }
    }

    fn process_attr_legacy(
        &mut self,
        node: &ruby_prism::CallNode<'_>,
        args: &[ruby_prism::Node<'_>],
    ) {
        if args.is_empty() {
            return;
        }
        let name = match extract_symbol_or_string(&args[0]) {
            Some(n) => n,
            None => return,
        };

        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();

        // Always readable
        self.found_method(&name, is_singleton, def_line, offset);

        // Writable if second arg is `true`
        if args.len() == 2 && args[1].as_true_node().is_some() {
            let setter = format!("{name}=");
            self.found_method(&setter, is_singleton, def_line, offset);
        }
    }

    fn process_def_delegator(
        &mut self,
        node: &ruby_prism::CallNode<'_>,
        args: &[ruby_prism::Node<'_>],
    ) {
        if args.len() < 2 {
            return;
        }
        // RuboCop requires the first arg to be a symbol or string (the target).
        // If it's a constant or other expression, skip entirely.
        if extract_symbol_or_string(&args[0]).is_none() {
            return;
        }
        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();

        if args.len() >= 3 {
            // Third arg is the alias name -- that's the method being defined
            if let Some(name) = extract_symbol_or_string(&args[2]) {
                self.found_method(&name, is_singleton, def_line, offset);
            }
        } else if let Some(name) = extract_symbol_or_string(&args[1]) {
            self.found_method(&name, is_singleton, def_line, offset);
        }
    }

    fn process_def_delegators(
        &mut self,
        node: &ruby_prism::CallNode<'_>,
        args: &[ruby_prism::Node<'_>],
    ) {
        if args.len() < 2 {
            return;
        }
        // RuboCop requires the first arg to be a symbol or string (the target).
        // If it's a constant or other expression, skip entirely.
        if extract_symbol_or_string(&args[0]).is_none() {
            return;
        }
        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();

        for arg in &args[1..] {
            if let Some(name) = extract_symbol_or_string(arg) {
                self.found_method(&name, is_singleton, def_line, offset);
            }
        }
    }

    /// Process `delegate :method1, :method2, to: :target` (ActiveSupport).
    /// RuboCop's delegate_method? pattern:
    ///   (send nil? :delegate ({sym str} $_)+ (hash <(pair (sym :to) {sym str}) ...>))
    /// The last arg must be a hash containing a `:to` key. All preceding args
    /// that are symbols or strings are method names being defined.
    /// Also handles `prefix: true` or `prefix: :name` which prepends `target_` or `name_`.
    fn process_delegate(&mut self, node: &ruby_prism::CallNode<'_>, args: &[ruby_prism::Node<'_>]) {
        if args.is_empty() {
            return;
        }

        // Last arg must be a keyword hash (e.g., `to: :target`)
        let last_arg = &args[args.len() - 1];
        let hash_node = match last_arg.as_keyword_hash_node() {
            Some(h) => h,
            None => return,
        };

        // Single scan: extract `:to` target and `:prefix` value
        let mut to_target: Option<String> = None;
        let mut prefix_is_true = false;
        let mut explicit_prefix: Option<String> = None;
        for element in hash_node.elements().iter() {
            if let Some(assoc) = element.as_assoc_node() {
                if let Some(key_sym) = assoc.key().as_symbol_node() {
                    let key_name = std::str::from_utf8(key_sym.unescaped()).unwrap_or("");
                    match key_name {
                        "to" => {
                            to_target = extract_symbol_or_string(&assoc.value());
                        }
                        "prefix" => {
                            let val = assoc.value();
                            if val.as_true_node().is_some() {
                                prefix_is_true = true;
                            } else {
                                explicit_prefix = extract_symbol_or_string(&val);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Must have a `:to` key
        if to_target.is_none() {
            return;
        }

        // Determine the effective prefix
        let name_prefix = if let Some(ep) = explicit_prefix {
            Some(ep)
        } else if prefix_is_true {
            to_target
        } else {
            None
        };

        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();

        // All args before the hash are method names
        for arg in &args[..args.len() - 1] {
            if let Some(name) = extract_symbol_or_string(arg) {
                let effective_name = if let Some(ref prefix) = name_prefix {
                    format!("{prefix}_{name}")
                } else {
                    name
                };
                self.found_method(&effective_name, is_singleton, def_line, offset);
            }
        }
    }
}

/// Extract a symbol value from a node (not strings).
fn extract_symbol_only(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(sym) = node.as_symbol_node() {
        return Some(
            std::str::from_utf8(sym.unescaped())
                .unwrap_or("")
                .to_string(),
        );
    }
    None
}

/// Extract a symbol or string value from a node.
fn extract_symbol_or_string(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(sym) = node.as_symbol_node() {
        return Some(
            std::str::from_utf8(sym.unescaped())
                .unwrap_or("")
                .to_string(),
        );
    }
    if let Some(s) = node.as_string_node() {
        return Some(std::str::from_utf8(s.unescaped()).unwrap_or("").to_string());
    }
    None
}

/// Check if a call node is a scope-creating pattern like `Class.new do`, `Module.new do`,
/// `Const.class_eval do`, or implicit `class_eval do`. Returns the scope name if so.
/// Note: Struct.new and module_eval are NOT scope-creating per RuboCop.
fn scope_creating_call_name(node: &ruby_prism::CallNode<'_>) -> Option<String> {
    let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");

    // class_eval with block (RuboCop only handles class_eval, not module_eval)
    if method_name == "class_eval" && node.block().is_some() {
        if let Some(recv) = node.receiver() {
            if let Some(const_read) = recv.as_constant_read_node() {
                let name = std::str::from_utf8(const_read.name().as_slice()).unwrap_or("");
                return Some(name.to_string());
            }
            if let Some(const_path) = recv.as_constant_path_node() {
                return Some(constant_path_name(&const_path));
            }
        }
        // class_eval with no explicit receiver (implicit self inside module)
        if node.receiver().is_none() {
            return Some("__implicit_class_eval__".to_string());
        }
    }

    // Class.new do / Module.new do
    if method_name == "new" && node.block().is_some() {
        if let Some(recv) = node.receiver() {
            if let Some(const_read) = recv.as_constant_read_node() {
                let name = std::str::from_utf8(const_read.name().as_slice()).unwrap_or("");
                if name == "Class" || name == "Module" {
                    return Some("__dynamic_class_new__".to_string());
                }
            }
        }
    }

    None
}

/// Build a full constant path name from a ConstantPathNode.
fn constant_path_name(node: &ruby_prism::ConstantPathNode<'_>) -> String {
    let child_name = node
        .name()
        .map(|n| std::str::from_utf8(n.as_slice()).unwrap_or(""))
        .unwrap_or("");

    if let Some(parent) = node.parent() {
        if let Some(parent_const) = parent.as_constant_read_node() {
            let parent_name = std::str::from_utf8(parent_const.name().as_slice()).unwrap_or("");
            return format!("{parent_name}::{child_name}");
        }
        if let Some(parent_path) = parent.as_constant_path_node() {
            return format!("{}::{child_name}", constant_path_name(&parent_path));
        }
    }
    child_name.to_string()
}

/// Extract the constant name from a singleton class expression (`class << ConstName`).
/// Returns Some(name) for `ConstantReadNode` or `ConstantPathNode`, None otherwise.
fn extract_sclass_const_name(expr: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(const_read) = expr.as_constant_read_node() {
        let name = std::str::from_utf8(const_read.name().as_slice()).unwrap_or("");
        return Some(name.to_string());
    }
    if let Some(const_path) = expr.as_constant_path_node() {
        return Some(constant_path_name(&const_path));
    }
    None
}

/// Apply RuboCop's humanization regex to a scope name containing `#<Class:...>`.
///
/// Matches two patterns:
/// 1. `SomeName::#<Class:SomeName>` → `SomeName.` (class << self inside module SomeName)
/// 2. `#<Class:SomeName>` → `SomeName.` (class << SomeName from outside)
///
/// Pattern 1 includes an optional `::` after the closing `>` to capture nested classes
/// like `#<Class:Foo>::Bar` → `Foo.Bar`.
fn humanize_scope(scope: &str) -> String {
    // Check for pattern 1: `Name::#<Class:Name>` where the names match
    // (mirrors the first alternative of RuboCop's regex)
    if let Some(class_start) = scope.find("#<Class:") {
        let before = &scope[..class_start];
        let after_class = &scope[class_start + 8..]; // skip "#<Class:"
        if let Some(close_pos) = after_class.find('>') {
            let class_name = &after_class[..close_pos];
            let remainder = &after_class[close_pos + 1..];
            // Strip optional leading "::" from remainder
            let remainder = remainder.strip_prefix("::").unwrap_or(remainder);

            // Check if `before` matches pattern `ClassName::` (first alternative)
            if let Some(stripped) = before.strip_suffix("::") {
                if stripped == class_name {
                    // Pattern 1: `Name::#<Class:Name>` → `Name.`
                    let result = format!("{stripped}.{remainder}");
                    return if result.ends_with('.') || remainder.is_empty() {
                        format!("{stripped}.")
                    } else {
                        format!("{stripped}.{remainder}")
                    };
                }
            }

            // Pattern 2: standalone `#<Class:Name>` (possibly with trailing content)
            if before.is_empty() {
                if remainder.is_empty() {
                    return format!("{class_name}.");
                }
                return format!("{class_name}.{remainder}");
            }
        }
    }
    scope.to_string()
}

impl<'pr> Visit<'pr> for DupMethodVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let name = class_or_module_name_from_constant(node.constant_path());
        self.scope_stack.push(ScopeEntry {
            name,
            is_singleton: false,
        });
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let name = class_or_module_name_from_constant(node.constant_path());
        self.scope_stack.push(ScopeEntry {
            name,
            is_singleton: false,
        });
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let expr = node.expression();

        if expr.as_self_node().is_some() {
            // `class << self` — mark the current scope as singleton, don't nest
            let depth = self.scope_stack.len();
            if depth > 0 {
                let was_singleton = self.scope_stack[depth - 1].is_singleton;
                self.scope_stack[depth - 1].is_singleton = true;
                if let Some(body) = node.body() {
                    self.visit(&body);
                }
                self.scope_stack[depth - 1].is_singleton = was_singleton;
            } else {
                // At top level, `class << self` creates Object singleton
                self.scope_stack.push(ScopeEntry {
                    name: "Object".to_string(),
                    is_singleton: true,
                });
                if let Some(body) = node.body() {
                    self.visit(&body);
                }
                self.scope_stack.pop();
            }
        } else if let Some(const_name) = extract_sclass_const_name(&expr) {
            // `class << SomeConst` or `class << A::B` — creates a singleton scope
            // for the constant. RuboCop's `parent_module_name` returns
            // `#<Class:ConstName>` which humanizes to `ConstName.` for methods.
            //
            // We push `#<Class:ConstName>` onto the scope stack (without clearing
            // existing scope). The `humanize_scope` function in `qualified_method_name`
            // converts this to `ConstName.` for direct methods, matching RuboCop's
            // output. For nested classes, the `#<Class:>` wrapper creates a distinct
            // key from the same class defined via `class << self` inside the module.
            //
            // The scope stack is saved/restored so reopened `class << Const` blocks
            // at different nesting levels share the same key prefix.
            let saved_scopes = self.scope_stack.clone();
            self.scope_stack.clear();
            self.scope_stack.push(ScopeEntry {
                name: format!("#<Class:{const_name}>"),
                is_singleton: false,
            });
            if let Some(body) = node.body() {
                self.visit(&body);
            }
            self.scope_stack = saved_scopes;
        } else if expr.as_call_node().is_some() {
            // `class << some_call_expr` (e.g., `class << Object.new`,
            // `class << @reflex.controller.response`)
            //
            // RuboCop's `found_sclass_method` handles defs inside sclass when
            // the sclass expression is a `send_type?` (method call). It tracks
            // methods as `receiver_method_name.method_name`. For example:
            //   class << Object.new
            //     def meth; end  → tracked as `new.meth`
            //   end
            //   class << @reflex.controller.response
            //     def body; end  → tracked as `response.body`
            //   end
            //
            // We extract the call's method name and use it as a pseudo-scope
            // so that defs inside are tracked and duplicates detected.
            let call = expr.as_call_node().unwrap();
            let recv_method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
            let saved_scopes = self.scope_stack.clone();
            self.scope_stack.clear();
            self.scope_stack.push(ScopeEntry {
                name: recv_method.to_string(),
                is_singleton: true,
            });
            if let Some(body) = node.body() {
                self.visit(&body);
            }
            // Also scan the sclass expression for DefNodes. RuboCop's
            // `found_sclass_method` catches defs inside the expression (e.g.,
            // `class << Class.new { def meth; end }.new`) because `parent_module_name`
            // returns nil and the sclass is found as an ancestor. We replicate this
            // by extracting def names from the expression and registering them
            // in the sclass scope.
            self.collect_defs_from_sclass_expr(&expr);
            self.scope_stack = saved_scopes;
        } else {
            // `class << @some_ivar` or other non-call, non-const, non-self expressions.
            // RuboCop's `found_sclass_method` only handles send_type? receivers,
            // so these are effectively invisible.
            self.plain_block_depth += 1;
            if let Some(body) = node.body() {
                self.visit(&body);
            }
            self.plain_block_depth -= 1;
        }
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.process_def(node);

        // Push def name for nested method scoping
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.def_stack.push(name);

        // Visit body for nested defs
        if let Some(body) = node.body() {
            self.visit(&body);
        }

        self.def_stack.pop();
    }

    fn visit_alias_method_node(&mut self, node: &ruby_prism::AliasMethodNode<'pr>) {
        self.process_alias(node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");

        // Check for `private def foo` / `protected def foo` pattern
        if matches!(method_name, "private" | "protected" | "public") {
            if let Some(arguments) = node.arguments() {
                let args: Vec<ruby_prism::Node<'pr>> = arguments.arguments().iter().collect();
                if args.len() == 1 {
                    if let Some(def_node) = args[0].as_def_node() {
                        self.process_def(&def_node);
                        // Visit the def body for nested methods
                        let name = std::str::from_utf8(def_node.name().as_slice())
                            .unwrap_or("")
                            .to_string();
                        self.def_stack.push(name);
                        if let Some(body) = def_node.body() {
                            self.visit(&body);
                        }
                        self.def_stack.pop();
                        return;
                    }
                }
            }
        }

        // Check for scope-creating calls (Class.new, class_eval, etc.)
        if let Some(scope_name) = scope_creating_call_name(node) {
            if scope_name == "__implicit_class_eval__" {
                // Implicit class_eval (no receiver) is transparent — stay in current scope.
                // In RuboCop, parent_module_name skips the block and finds the enclosing
                // class/module. So we just visit the block body without scope changes.
                if let Some(block) = node.block() {
                    self.visit(&block);
                }
                return;
            }
            let effective_name = if scope_name == "__dynamic_class_new__" {
                // Local variable assignment -- isolated scope per assignment
                // Constant assignment is handled by visit_constant_write_node.
                if let Some(block) = node.block() {
                    let saved_defs = std::mem::take(&mut self.definitions);
                    let saved_scopes = self.scope_stack.clone();
                    self.scope_stack.clear();
                    self.scope_stack.push(ScopeEntry {
                        name: "__anonymous__".to_string(),
                        is_singleton: false,
                    });
                    self.visit(&block);
                    self.definitions = saved_defs;
                    self.scope_stack = saved_scopes;
                }
                return;
            } else {
                scope_name
            };

            self.scope_stack.push(ScopeEntry {
                name: effective_name,
                is_singleton: false,
            });
            if let Some(block) = node.block() {
                self.visit(&block);
            }
            self.scope_stack.pop();
            return;
        }

        // Check for alias_method, attr_*, def_delegator* calls
        self.process_call(node);

        // Visit children
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(arguments) = node.arguments() {
            for arg in arguments.arguments().iter() {
                self.visit(&arg);
            }
        }
        // Visit block as a plain (non-scope-creating) block.
        // Methods inside these blocks are ignored per RuboCop behavior.
        if let Some(block) = node.block() {
            self.plain_block_depth += 1;
            self.visit(&block);
            self.plain_block_depth -= 1;
        }
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        let value = node.value();
        // Check for `A = Class.new do ... end` or `A = Module.new do ... end`
        if let Some(call) = value.as_call_node() {
            let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
            if method_name == "new" && call.block().is_some() {
                if let Some(recv) = call.receiver() {
                    if let Some(const_read) = recv.as_constant_read_node() {
                        let recv_name =
                            std::str::from_utf8(const_read.name().as_slice()).unwrap_or("");
                        if recv_name == "Class" || recv_name == "Module" {
                            let const_name =
                                std::str::from_utf8(node.name().as_slice()).unwrap_or("");
                            self.scope_stack.push(ScopeEntry {
                                name: const_name.to_string(),
                                is_singleton: false,
                            });
                            if let Some(block) = call.block() {
                                self.visit(&block);
                            }
                            self.scope_stack.pop();
                            return;
                        }
                    }
                }
            }
        }
        // Default: visit children
        self.visit(&value);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.if_depth += 1;
        ruby_prism::visit_if_node(self, node);
        self.if_depth -= 1;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.if_depth += 1;
        ruby_prism::visit_unless_node(self, node);
        self.if_depth -= 1;
    }

    // NOTE: case/case_match nodes do NOT suppress duplicate detection.
    // RuboCop's `node.each_ancestor.any?(&:if_type?)` only matches if/unless,
    // not case/when. So methods inside case/when ARE checked for duplicates.

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        let was = self.in_rescue_or_ensure;
        self.in_rescue_or_ensure = true;
        self.rescue_ensure_type_stack.push(RescueEnsureType::Rescue);
        ruby_prism::visit_rescue_node(self, node);
        self.rescue_ensure_type_stack.pop();
        self.in_rescue_or_ensure = was;
    }

    fn visit_ensure_node(&mut self, node: &ruby_prism::EnsureNode<'pr>) {
        let was = self.in_rescue_or_ensure;
        self.in_rescue_or_ensure = true;
        self.rescue_ensure_type_stack.push(RescueEnsureType::Ensure);
        ruby_prism::visit_ensure_node(self, node);
        self.rescue_ensure_type_stack.pop();
        self.in_rescue_or_ensure = was;
    }
}

/// Extract the name from a class/module constant path.
fn class_or_module_name_from_constant(constant_path: ruby_prism::Node<'_>) -> String {
    if let Some(const_read) = constant_path.as_constant_read_node() {
        return std::str::from_utf8(const_read.name().as_slice())
            .unwrap_or("")
            .to_string();
    }
    if let Some(const_path) = constant_path.as_constant_path_node() {
        return constant_path_name(&const_path);
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateMethods, "cops/lint/duplicate_methods");

    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    fn count_offenses(source: &[u8]) -> usize {
        run_cop_full(&DuplicateMethods, source).len()
    }

    // Hypothesis tests for identifying FP/FN root causes

    #[test]
    fn test_rescue_ensure_global_scope() {
        // RuboCop uses a global per-type scope for rescue/ensure.
        // Nested defs inside `setup` are scoped as "setup.Foo#bar"
        // so the two defs inside setup are the same key. Rescue forgives.
        let n = count_offenses(b"class Foo\n  def bar; 1; end\n\n  def setup\n    def bar; 2; end\n  rescue\n    def bar; 3; end\n  end\nend\n");
        assert_eq!(n, 0, "rescue should forgive first redefinition");
    }

    #[test]
    fn test_struct_new_not_scope() {
        // Struct.new blocks are NOT recognized as scope-creating by RuboCop
        // (only Class.new and Module.new are). Defs inside are ignored since
        // parent_module_name returns nil for unrecognized blocks.
        let n =
            count_offenses(b"A = Struct.new(:x) do\n  def foo; 1; end\n  def foo; 2; end\nend\n");
        assert_eq!(n, 0, "Struct.new blocks should be ignored (no scope)");
    }

    #[test]
    fn test_class_eval_with_string() {
        // class_eval with a string argument (not block) should not create scope
        let n = count_offenses(
            b"class Foo\n  def bar; 1; end\nend\nFoo.class_eval(\"def baz; end\")\n",
        );
        assert_eq!(n, 0, "class_eval with string should not create scope");
    }

    #[test]
    fn test_module_eval_not_scope() {
        // module_eval is NOT recognized as scope-creating by RuboCop (only class_eval is).
        // Defs inside module_eval blocks are ignored since parent_module_name returns nil.
        let n = count_offenses(b"Foo.module_eval do\n  def bar; 1; end\n  def bar; 2; end\nend\n");
        assert_eq!(
            n, 0,
            "module_eval blocks should be ignored (not scope-creating)"
        );
    }

    #[test]
    fn test_class_open_constant_path() {
        // class A::B should be handled
        let n = count_offenses(b"class A::B\n  def foo; 1; end\n  def foo; 2; end\nend\n");
        assert_eq!(n, 1, "class with constant path should detect duplicates");
    }

    #[test]
    fn test_dup_inside_class_with_block() {
        // Methods inside `included do` or similar blocks should be ignored
        let n = count_offenses(
            b"class Foo\n  included do\n    def bar; 1; end\n    def bar; 2; end\n  end\nend\n",
        );
        assert_eq!(n, 0, "methods in DSL blocks should be ignored");
    }

    #[test]
    fn test_sclass_inside_sclass() {
        // class << self inside class has nested class B - different scopes
        let n = count_offenses(b"class Foo\n  class << self\n    def bar; 1; end\n\n    class B\n      def bar; 2; end\n    end\n  end\nend\n");
        assert_eq!(n, 0, "different scopes should not conflict");
    }

    #[test]
    fn test_rescue_ensure_rubocop_behavior() {
        // Ensure scope should forgive first redefinition of alias_method :save
        let n = count_offenses(b"module FooTest\n  def make_save_always_fail\n    Foo.class_eval do\n      def failed_save\n        raise\n      end\n      alias_method :original_save, :save\n      alias_method :save, :failed_save\n    end\n\n    yield\n  ensure\n    Foo.class_eval do\n      alias_method :save, :original_save\n    end\n  end\nend\n");
        assert_eq!(n, 0, "ensure scope should forgive first redefinition");
    }

    #[test]
    fn test_rescue_global_type_scope() {
        // RuboCop: @scopes[:rescue] is shared across ALL rescue blocks.
        // Second rescue should NOT forgive a method already forgiven by first rescue.
        let n = count_offenses(b"class Foo\n  def bar; 1; end\n\n  def test1\n    def bar; 2; end\n  rescue\n    def bar; 3; end\n  end\n\n  def test2\n    x = 1\n  rescue\n    def bar; 4; end\n  end\nend\n");
        // bar is defined at line 2. In test1: nested bar is "test1.Foo#bar" (different key).
        // In test2: nested bar is "test2.Foo#bar" (different key). No conflicts.
        assert_eq!(n, 0, "different enclosing methods create different keys");
    }

    #[test]
    fn test_multiple_rescue_same_method() {
        // In the same enclosing scope (no def wrapper), multiple rescue blocks share type scope
        // RuboCop uses @scopes[:rescue] globally - first forgiven, second is offense
        let n = count_offenses(b"class Foo\n  def bar; 1; end\n\n  begin\n    x = 1\n  rescue\n    def bar; 2; end\n  end\n\n  begin\n    y = 1\n  rescue\n    def bar; 3; end\n  end\nend\n");
        // bar defined at line 2. First rescue: bar redef forgiven. Second rescue: offense.
        // RuboCop: @scopes[:rescue] = ["Foo#bar"] after first rescue
        // Second rescue: key "Foo#bar" already in @scopes[:rescue], so offense.
        assert_eq!(
            n, 1,
            "second rescue should report offense (global type scope)"
        );
    }

    #[test]
    fn test_constant_path_class_name() {
        // class A::B::C should produce scope "A::B::C"
        let n = count_offenses(b"class A::B::C\n  def foo; 1; end\n  def foo; 2; end\nend\n");
        assert_eq!(
            n, 1,
            "class with deep constant path should detect duplicates"
        );
    }

    #[test]
    fn test_local_var_struct_new_isolated() {
        // local = Struct.new do ... end should isolate scope (like Class.new)
        let n = count_offenses(b"a = Struct.new(:x) do\n  def foo; 1; end\nend\nb = Struct.new(:x) do\n  def foo; 2; end\nend\n");
        assert_eq!(n, 0, "local var Struct.new should isolate scopes");
    }

    #[test]
    fn test_constant_write_struct_new_separate() {
        // Two different constants with Struct.new should have separate scopes
        let n = count_offenses(b"A = Struct.new(:x) do\n  def foo; 1; end\nend\nB = Struct.new(:y) do\n  def foo; 2; end\nend\n");
        assert_eq!(n, 0, "separate Struct.new constants have separate scopes");
    }

    #[test]
    fn test_reopened_struct_new_no_scope() {
        // Struct.new blocks are not scope-creating, so no dups detected
        let n = count_offenses(b"A = Struct.new(:x) do\n  def foo; 1; end\nend\nA = Struct.new(:y) do\n  def foo; 2; end\nend\n");
        assert_eq!(n, 0, "Struct.new blocks are not scope-creating");
    }

    #[test]
    fn test_case_does_not_suppress() {
        // RuboCop only suppresses if/unless ancestors, NOT case/when.
        // Methods inside case/when ARE checked for duplicates.
        let n = count_offenses(b"class Foo\n  case RUBY_VERSION\n  when '3.0'\n    def bar; 1; end\n  when '2.7'\n    def bar; 2; end\n  end\nend\n");
        assert_eq!(n, 1, "case/when should NOT suppress duplicate detection");
    }

    #[test]
    fn test_implicit_class_eval_transparent() {
        // class_eval with no receiver inside a module is transparent — defs
        // are scoped to the enclosing module, not a nested "A::A" scope.
        let n = count_offenses(
            b"module A\n  class_eval do\n    def foo; 1; end\n    def foo; 2; end\n  end\nend\n",
        );
        assert_eq!(n, 1, "implicit class_eval should use enclosing scope");
    }

    #[test]
    fn test_def_inside_for_loop() {
        // Methods inside for loops should NOT be suppressed by nitrocop
        // (RuboCop only suppresses if_type? ancestors, not for loops)
        let n = count_offenses(
            b"class Foo\n  def bar; 1; end\n  for x in items\n    def bar; 2; end\n  end\nend\n",
        );
        assert_eq!(n, 1, "for loop should not suppress duplicate detection");
    }

    #[test]
    fn test_class_eval_constant_path() {
        // A::B.class_eval do should create scope "A::B"
        let n = count_offenses(b"A::B.class_eval do\n  def foo; 1; end\n  def foo; 2; end\nend\n");
        assert_eq!(
            n, 1,
            "class_eval with constant path should detect duplicates"
        );
    }

    #[test]
    fn test_constant_write_with_non_new_method() {
        // A = SomeThing.build do...end should NOT create scope
        // Only Class.new/Module.new create scopes
        let n =
            count_offenses(b"A = SomeThing.build do\n  def foo; 1; end\n  def foo; 2; end\nend\n");
        // This is a plain block — methods inside should be ignored
        assert_eq!(n, 0, "non-new method blocks should be ignored");
    }

    #[test]
    fn test_class_reopened_with_sclass() {
        // Reopened class with class << self should detect dups
        let n = count_offenses(b"class A\n  class << self\n    def foo; 1; end\n  end\nend\nclass A\n  class << self\n    def foo; 2; end\n  end\nend\n");
        assert_eq!(n, 1, "reopened class with sclass should detect dups");
    }

    #[test]
    fn test_mixed_def_self_and_sclass() {
        // def self.foo and class << self; def foo should be same scope
        let n = count_offenses(
            b"class A\n  def self.foo; 1; end\n  class << self\n    def foo; 2; end\n  end\nend\n",
        );
        assert_eq!(n, 1, "def self.foo and sclass def foo should be same scope");
    }

    #[test]
    fn test_def_const_outer_scope() {
        // def M.foo inside class A within module M should resolve to M.foo
        let n = count_offenses(
            b"module M\n  class A\n    def M.foo; 1; end\n    def M.foo; 2; end\n  end\nend\n",
        );
        assert_eq!(n, 1, "def M.foo should resolve M to outer scope");
    }

    #[test]
    fn test_def_const_and_self_same_class() {
        // def A.foo and def self.foo inside class A should be same
        let n = count_offenses(b"class A\n  def A.foo; 1; end\n  def self.foo; 2; end\nend\n");
        assert_eq!(n, 1, "def A.foo and def self.foo should be same");
    }

    #[test]
    fn test_alias_method_string_args_not_tracked() {
        // RuboCop's alias_method? pattern only matches symbol args, not strings.
        // alias_method "foo", "bar" after alias_method :foo, :bar should NOT be an offense.
        let n = count_offenses(
            b"class Foo\n  alias_method :process, :other\n  alias_method \"process\", \"other\"\nend\n",
        );
        assert_eq!(n, 0, "alias_method with string args should not be tracked");
    }

    fn count_offenses_with_active_support(source: &[u8]) -> usize {
        let mut config = CopConfig::default();
        config
            .options
            .insert("ActiveSupportExtensionsEnabled".to_string(), true.into());
        run_cop_full_with_config(&DuplicateMethods, source, config).len()
    }

    #[test]
    fn test_delegate_with_active_support() {
        // delegate :method, to: :target followed by def method should be an offense
        // when ActiveSupportExtensionsEnabled is true
        let n = count_offenses_with_active_support(
            b"class Foo\n  delegate :process, to: :target\n  def process; end\nend\n",
        );
        assert_eq!(n, 1, "delegate + def should be offense with ActiveSupport");
    }

    #[test]
    fn test_delegate_without_active_support() {
        // delegate should NOT be tracked without ActiveSupportExtensionsEnabled
        let n = count_offenses(
            b"class Foo\n  delegate :process, to: :target\n  def process; end\nend\n",
        );
        assert_eq!(n, 0, "delegate should not be tracked without ActiveSupport");
    }

    #[test]
    fn test_delegate_duplicate_in_same_call() {
        // delegate :foo, :foo, to: :bar — same method listed twice
        let n = count_offenses_with_active_support(
            b"class Foo\n  delegate :bar, :bar, to: :target\nend\n",
        );
        assert_eq!(n, 1, "duplicate symbol in same delegate call");
    }

    #[test]
    fn test_delegate_multiple_calls_same_method() {
        // Two separate delegate calls defining the same method
        let n = count_offenses_with_active_support(
            b"class Foo\n  delegate :bar, to: :target1\n  delegate :bar, to: :target2\nend\n",
        );
        assert_eq!(n, 1, "same method in two delegate calls");
    }

    #[test]
    fn test_delegate_with_prefix_true() {
        // delegate :name, to: :target, prefix: true => defines target_name
        let n = count_offenses_with_active_support(
            b"class Foo\n  delegate :name, to: :target, prefix: true\n  def target_name; end\nend\n",
        );
        assert_eq!(n, 1, "delegate with prefix: true should prepend target_");
    }

    #[test]
    fn test_delegate_with_prefix_symbol() {
        // delegate :name, to: :target, prefix: :custom => defines custom_name
        let n = count_offenses_with_active_support(
            b"class Foo\n  delegate :name, to: :target, prefix: :custom\n  def custom_name; end\nend\n",
        );
        assert_eq!(n, 1, "delegate with prefix: :custom should prepend custom_");
    }

    #[test]
    fn test_delegate_inside_condition() {
        // delegate inside if/unless should be ignored
        let n = count_offenses_with_active_support(
            b"class Foo\n  def process; end\n  if cond\n    delegate :process, to: :bar\n  end\nend\n",
        );
        assert_eq!(n, 0, "delegate inside condition should be ignored");
    }

    #[test]
    fn test_delegate_no_to_key() {
        // delegate without :to key should be ignored
        let n = count_offenses_with_active_support(
            b"class Foo\n  delegate :process\n  def process; end\nend\n",
        );
        assert_eq!(n, 0, "delegate without :to should be ignored");
    }

    #[test]
    fn test_sclass_constant_path_detects_dups() {
        // RuboCop's parent_module_name returns `#<Class:Multiton::ClassMethods>` for
        // defs inside `class << Multiton::ClassMethods`, humanized to
        // `Multiton::ClassMethods.`. Duplicate defs ARE detected.
        let n = count_offenses(
            b"class << Multiton::ClassMethods\n  def extended; 1; end\n  def extended; 2; end\nend\n",
        );
        assert_eq!(n, 1, "class << A::B should detect duplicate defs");
    }

    #[test]
    fn test_sclass_constant_path_reopened_detects_dups() {
        // Reopened class << A::B shares the same scope — duplicates detected
        let n = count_offenses(
            b"class << Multiton::ClassMethods\n  def extended; 1; end\nend\nclass << Multiton::ClassMethods\n  def extended; 2; end\nend\n",
        );
        assert_eq!(n, 1, "reopened class << A::B should detect dups");
    }

    #[test]
    fn test_sclass_const_nested_class_no_fp() {
        // Nested class inside `class << Const` should NOT conflict with
        // the same class inside `module Const > class << self`.
        // RuboCop produces different scope keys for these two contexts.
        let n = count_offenses(
            b"class << Multiton\n  class Nested\n    def init; end\n  end\nend\nmodule Multiton\n  class << self\n    class Nested\n      def init; end\n    end\n  end\nend\n",
        );
        assert_eq!(
            n, 0,
            "nested class in sclass const vs sclass self should not conflict"
        );
    }

    #[test]
    fn test_sclass_const_cross_ref_with_def_self() {
        // `def self.foo` inside `module M` and `def foo` inside `class << M` should match
        let n = count_offenses(
            b"module Container\n  def self.helper; 1; end\nend\nclass << Container\n  def helper; 2; end\nend\n",
        );
        assert_eq!(
            n, 1,
            "class << Const should cross-reference with def self.method"
        );
    }

    #[test]
    fn test_def_const_method_inside_block_with_scope() {
        // `def ConstName.method` inside a block within the module that defines ConstName
        // should detect duplicates — the constant resolves through the scope stack.
        let n = count_offenses(
            b"module M\n  def self.foo; 1; end\n  dsl_block do\n    def M.foo; 2; end\n  end\nend\n",
        );
        assert_eq!(
            n, 1,
            "def ConstName.method inside block within same module should detect dup"
        );
    }

    #[test]
    fn test_def_const_method_inside_nested_blocks() {
        // `def ConstName.method` inside deeply nested blocks should still resolve
        // the constant if it's in the scope stack.
        let n = count_offenses(
            b"class Base\n  class << self\n    def all; []; end\n    namespace do\n      task do\n        def Base.all; 1; end\n      end\n    end\n  end\nend\n",
        );
        assert_eq!(
            n, 1,
            "def ConstName.method inside nested blocks should detect dup"
        );
    }

    #[test]
    fn test_def_self_method_inside_block_ignored() {
        // def self.method inside a DSL block is ignored (parent_module_name returns nil)
        let n = count_offenses(
            b"class Foo\n  def self.bar; 1; end\nend\ndsl_block do\n  def self.bar; 2; end\nend\n",
        );
        assert_eq!(n, 0, "def self.method inside block should be ignored");
    }

    #[test]
    fn test_delegate_then_def_with_active_support() {
        // delegate :method then def method should be an offense
        let n = count_offenses_with_active_support(
            b"class Foo\n  delegate :run, to: :target\n  def run; end\nend\n",
        );
        assert_eq!(n, 1, "delegate then def should be offense");
    }

    #[test]
    fn test_delegate_then_delegate_same_method() {
        // Two delegate calls defining same method via different targets
        let n = count_offenses_with_active_support(
            b"class Foo\n  delegate :status, to: :adapter\n  delegate :status, to: :model\nend\n",
        );
        assert_eq!(n, 1, "two delegates defining same method should be offense");
    }

    #[test]
    fn test_attr_accessor_then_delegate_same_method() {
        // attr_accessor defines reader+writer, delegate defines reader — dup on reader
        let n = count_offenses_with_active_support(
            b"class Foo\n  attr_accessor :changes, :errors\n  delegate :changes, :errors, to: :@value\nend\n",
        );
        assert_eq!(
            n, 2,
            "attr_accessor then delegate same methods should be offense"
        );
    }

    #[test]
    fn test_delegate_inside_sclass_self() {
        // delegate inside class << self should work with ActiveSupport
        let n = count_offenses_with_active_support(
            b"class Foo\n  class << self\n    delegate :all, :find, to: :resource\n    def all; []; end\n  end\nend\n",
        );
        assert_eq!(
            n, 1,
            "delegate then def inside class << self should detect dup"
        );
    }
}
