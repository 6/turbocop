//! Curated pattern database for verifier.
//!
//! Contains (cop_name, pattern_string) pairs extracted from vendor Ruby source.
//! These patterns are used by the verifier to test the interpreter against
//! real-world NodePattern definitions from RuboCop and its plugins.
//!
//! Patterns are classified by feature level:
//! - Phase 1: uses only supported features (node types, wildcards, rest,
//!   nil?, symbols, ints, alternatives, negation, capture, type predicates)
//! - Has helpers: contains `#method` calls (always-true in interpreter)
//! - Has params: contains `%param` references (always-true in interpreter)

/// A (cop_name, pattern_string) pair for verifier matching.
pub struct PatternEntry {
    pub cop_name: &'static str,
    pub pattern: &'static str,
}

/// Curated set of vendor NodePattern definitions.
///
/// ~50 patterns covering common cops across Style, Lint, Performance, and Rails.
/// Patterns containing `#` (helper calls) or `%` (param refs) are included but
/// those features evaluate as always-true in the Phase 1 interpreter.
pub const PATTERNS: &[PatternEntry] = &[
    // ── Lint ──────────────────────────────────────────────────────────────
    PatternEntry {
        cop_name: "Lint/BooleanSymbol",
        pattern: "(sym {:true :false})",
    },
    PatternEntry {
        cop_name: "Lint/RandOne",
        pattern: "(send {(const {nil? cbase} :Kernel) nil?} :rand {(int {-1 1}) (float {-1.0 1.0})})",
    },
    PatternEntry {
        cop_name: "Lint/SafeNavigationChain",
        pattern: "{(send $(csend ...) $_ ...) (send $(any_block (csend ...) ...) $_ ...)}",
    },
    PatternEntry {
        cop_name: "Lint/RedundantStringCoercion",
        pattern: "(call _ :to_s)",
    },
    PatternEntry {
        cop_name: "Lint/UselessAccessModifier/static",
        pattern: "{def (send nil? {:attr :attr_reader :attr_writer :attr_accessor} ...)}",
    },
    PatternEntry {
        cop_name: "Lint/UselessAccessModifier/dynamic",
        pattern: "{(send nil? :define_method ...) (any_block (send nil? :define_method ...) ...)}",
    },
    PatternEntry {
        cop_name: "Lint/UselessAccessModifier/eval",
        pattern: "(any_block (send _ {:class_eval :instance_eval}) ...)",
    },
    // ── Style ─────────────────────────────────────────────────────────────
    PatternEntry {
        cop_name: "Style/NilComparison/comparison",
        pattern: "(send _ {:== :===} nil)",
    },
    PatternEntry {
        cop_name: "Style/NilComparison/check",
        pattern: "(send _ :nil?)",
    },
    PatternEntry {
        cop_name: "Style/DoubleNegation",
        pattern: "(send (send _ :!) :!)",
    },
    PatternEntry {
        cop_name: "Style/NumericPredicate/predicate",
        pattern: "(send $(...) ${:zero? :positive? :negative?})",
    },
    PatternEntry {
        cop_name: "Style/NumericPredicate/comparison",
        pattern: "(send [$(...) !gvar_type?] ${:== :> :<} (int 0))",
    },
    PatternEntry {
        cop_name: "Style/NumericPredicate/inverted",
        pattern: "(send (int 0) ${:== :> :<} [$(...) !gvar_type?])",
    },
    PatternEntry {
        cop_name: "Style/SymbolProc/proc_node",
        pattern: "(send (const {nil? cbase} :Proc) :new)",
    },
    PatternEntry {
        cop_name: "Style/StringConcatenation",
        pattern: "{(send str_type? :+ _) (send _ :+ str_type?)}",
    },
    PatternEntry {
        cop_name: "Style/Strip",
        pattern: "{(call $(call _ :rstrip) :lstrip) (call $(call _ :lstrip) :rstrip)}",
    },
    PatternEntry {
        cop_name: "Style/EvenOdd",
        pattern: "(send {(send $_ :% (int 2)) (begin (send $_ :% (int 2)))} ${:== :!=} (int ${0 1}))",
    },
    PatternEntry {
        cop_name: "Style/CollectionCompact/reject_block_pass",
        pattern: "(call !nil? {:reject :reject!} (block_pass (sym :nil?)))",
    },
    PatternEntry {
        cop_name: "Style/CollectionCompact/grep_v_nil",
        pattern: "(send _ :grep_v {(nil) (const {nil? cbase} :NilClass)})",
    },
    PatternEntry {
        cop_name: "Style/MapToHash",
        pattern: "{$(call (any_block $(call _ {:map :collect}) ...) :to_h) $(call $(call _ {:map :collect} (block_pass sym)) :to_h)}",
    },
    PatternEntry {
        cop_name: "Style/RedundantFreeze",
        pattern: "{(begin (send {float int} {:+ :- :* :** :/ :% :<<} _)) (begin (send !{(str _) array} {:+ :- :* :** :/ :%} {float int})) (begin (send _ {:== :=== :!= :<= :>= :< :>} _)) (send _ {:count :length :size} ...) (any_block (send _ {:count :length :size} ...) ...)}",
    },
    PatternEntry {
        cop_name: "Style/RedundantSort",
        pattern: "{(call $(call _ $:sort) ${:last :first}) (call $(call _ $:sort) ${:[] :at :slice} {(int 0) (int -1)})}",
    },
    // ── Performance ───────────────────────────────────────────────────────
    PatternEntry {
        cop_name: "Performance/ReverseEach",
        pattern: "(call (call _ :reverse) :each)",
    },
    PatternEntry {
        cop_name: "Performance/Count",
        pattern: "{(call (block $(call _ ${:select :filter :find_all :reject}) ...) ${:count :length :size}) (call $(call _ ${:select :filter :find_all :reject} (:block_pass _)) ${:count :length :size})}",
    },
    PatternEntry {
        cop_name: "Performance/FlatMap",
        pattern: "(call {$(block (call _ ${:collect :map}) ...) $(call _ ${:collect :map} (block_pass _))} ${:flatten :flatten!} $...)",
    },
    PatternEntry {
        cop_name: "Performance/StringReplacement",
        pattern: "(call _ {:gsub :gsub!} ${regexp str (send (const nil? :Regexp) {:new :compile} _)} $str)",
    },
    PatternEntry {
        cop_name: "Performance/CompareWithBlock",
        pattern: "(block $(send _ {:sort :sort! :min :max :minmax}) (args (arg $_a) (arg $_b)) $send)",
    },
    PatternEntry {
        cop_name: "Performance/Squeeze",
        pattern: "(call $!nil? ${:gsub :gsub!} (regexp (str $#repeating_literal?) (regopt)) (str $_))",
    },
    PatternEntry {
        cop_name: "Performance/Size/array",
        pattern: "{[!nil? array_type?] (call _ :to_a) (send (const nil? :Array) :[] _) (send nil? :Array _)}",
    },
    PatternEntry {
        cop_name: "Performance/Size/hash",
        pattern: "{[!nil? hash_type?] (call _ :to_h) (send (const nil? :Hash) :[] _) (send nil? :Hash _)}",
    },
    // ── Additional simple patterns from various cops ──────────────────────
    PatternEntry {
        cop_name: "Lint/RedundantRequireStatement",
        pattern: "(send nil? :require (str #redundant_feature?))",
    },
    PatternEntry {
        cop_name: "Style/SymbolProc/receiver",
        pattern: "{(call ...) (super ...) zsuper}",
    },
    PatternEntry {
        cop_name: "Naming/BinaryOperatorParameterName",
        pattern: "(def [#op_method? $_] (args $(arg [!:other !:_other])) _)",
    },
    PatternEntry {
        cop_name: "Style/YodaCondition",
        pattern: "(send #source_file_path_constant? {:== :!=} (gvar #program_name?))",
    },
    // ── Patterns testing specific interpreter features ────────────────────
    // Simple wildcard + symbol
    PatternEntry {
        cop_name: "_test/simple_send",
        pattern: "(send _ :foo)",
    },
    // Nil receiver
    PatternEntry {
        cop_name: "_test/nil_receiver_send",
        pattern: "(send nil? :require ...)",
    },
    // Nested send
    PatternEntry {
        cop_name: "_test/nested_send",
        pattern: "(send (send _ :where) :first)",
    },
    // Negation
    PatternEntry {
        cop_name: "_test/negated_nil",
        pattern: "(send !nil? :foo)",
    },
    // Type predicate at top level
    PatternEntry {
        cop_name: "_test/type_predicate",
        pattern: "str?",
    },
    // Alternatives at top level
    PatternEntry {
        cop_name: "_test/alternatives",
        pattern: "{send? csend?}",
    },
    // Conjunction
    PatternEntry {
        cop_name: "_test/conjunction",
        pattern: "[!nil? send?]",
    },
    // Capture
    PatternEntry {
        cop_name: "_test/capture",
        pattern: "$(send _ :foo)",
    },
    // Int literal in node
    PatternEntry {
        cop_name: "_test/int_value",
        pattern: "(int 42)",
    },
    // String literal in node
    PatternEntry {
        cop_name: "_test/str_value",
        pattern: "(str \"hello\")",
    },
    // Symbol value in node
    PatternEntry {
        cop_name: "_test/sym_value",
        pattern: "(sym :foo)",
    },
    // Boolean literals
    PatternEntry {
        cop_name: "_test/true_literal",
        pattern: "true",
    },
    PatternEntry {
        cop_name: "_test/false_literal",
        pattern: "false",
    },
    PatternEntry {
        cop_name: "_test/nil_literal",
        pattern: "nil",
    },
    // If with absent else
    PatternEntry {
        cop_name: "_test/if_no_else",
        pattern: "(if _ _ nil?)",
    },
    // Def pattern
    PatternEntry {
        cop_name: "_test/def_pattern",
        pattern: "(def :initialize ...)",
    },
    // Array rest
    PatternEntry {
        cop_name: "_test/array_rest",
        pattern: "(array ...)",
    },
];
