use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// RSpec/DescribedClass: Use `described_class` instead of referencing the class directly.
pub struct DescribedClass;

impl Cop for DescribedClass {
    fn name(&self) -> &'static str {
        "RSpec/DescribedClass"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        let skip_blocks = config.get_bool("SkipBlocks", false);
        let enforced_style = config.get_str("EnforcedStyle", "described_class");
        let only_static = config.get_bool("OnlyStaticConstants", true);

        let mut visitor = DescribedClassVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            // The resolved full name of the described class (namespace + const name collapsed)
            described_full_name: None,
            // The raw source text of the described class argument (for messages)
            described_class_source: None,
            enforced_style: enforced_style.to_string(),
            skip_blocks,
            only_static_constants: only_static,
            in_scope_change: false,
            // Track enclosing namespace from module/class nodes
            namespace: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// A segment in a constant name. `None` represents a rooted constant (cbase, `::Foo`).
type ConstName = Vec<Option<Vec<u8>>>;

struct DescribedClassVisitor<'a> {
    cop: &'a DescribedClass,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// The fully resolved name of the described class (namespace-collapsed).
    described_full_name: Option<ConstName>,
    /// The raw source text of the described class (for error messages).
    described_class_source: Option<Vec<u8>>,
    enforced_style: String,
    skip_blocks: bool,
    only_static_constants: bool,
    in_scope_change: bool,
    /// Enclosing namespace segments from module/class nodes outside describe blocks.
    namespace: Vec<Vec<u8>>,
}

/// Extract const_name segments from a Prism node.
/// Returns `None` if the node is not a constant reference.
/// - `Foo` => `[Some("Foo")]`
/// - `Foo::Bar` => `[Some("Foo"), Some("Bar")]`
/// - `::Foo` => `[None, Some("Foo")]`
/// - `(expr)::Foo` => `[None, Some("Foo")]` (non-const parent treated as rooted)
fn extract_const_name(node: &ruby_prism::Node<'_>) -> Option<ConstName> {
    if let Some(cr) = node.as_constant_read_node() {
        return Some(vec![Some(cr.name().as_slice().to_vec())]);
    }
    if let Some(ref cp) = node.as_constant_path_node() {
        return extract_const_name_from_path(cp);
    }
    None
}

fn extract_const_name_from_path(node: &ruby_prism::ConstantPathNode<'_>) -> Option<ConstName> {
    let name_bytes = node.name()?.as_slice().to_vec();
    if let Some(parent) = node.parent() {
        if let Some(cr) = parent.as_constant_read_node() {
            return Some(vec![Some(cr.name().as_slice().to_vec()), Some(name_bytes)]);
        }
        if let Some(ref cp) = parent.as_constant_path_node() {
            let mut segments = extract_const_name_from_path(cp)?;
            segments.push(Some(name_bytes));
            return Some(segments);
        }
        // Non-constant parent (e.g., `(expr)::Foo`, `foo::Bar`) => [None, name]
        return Some(vec![None, Some(name_bytes)]);
    }
    // No parent means rooted `::Foo`
    Some(vec![None, Some(name_bytes)])
}

/// RuboCop's `collapse_namespace` algorithm.
///
/// Merges a namespace prefix with a constant name, handling overlapping segments.
///
/// Examples:
/// - `collapse_namespace([], [C])` => `[C]`
/// - `collapse_namespace([A, B], [C])` => `[A, B, C]`
/// - `collapse_namespace([A, B], [B, C])` => `[A, B, C]`
/// - `collapse_namespace([A, B], [None, C])` => `[None, C]` (rooted const)
/// - `collapse_namespace([A, B], [None, B, C])` => `[None, B, C]` (rooted const)
fn collapse_namespace(namespace: &[Vec<u8>], const_name: &ConstName) -> ConstName {
    // If namespace is empty or constant is rooted (starts with None), return const as-is
    if namespace.is_empty() || const_name.first().is_some_and(|s| s.is_none()) {
        return const_name.clone();
    }

    let ns_len = namespace.len();
    let c_len = const_name.len();

    let start = ns_len.saturating_sub(c_len);
    let max = ns_len;

    // Find the first shift where namespace[shift..] matches const_name[..max-shift]
    let intersection = (start..=max)
        .find(|&shift| {
            let ns_slice = &namespace[shift..max];
            let c_slice = &const_name[..max - shift];
            ns_slice.len() == c_slice.len()
                && ns_slice
                    .iter()
                    .zip(c_slice.iter())
                    .all(|(ns_seg, c_seg)| c_seg.as_ref().is_some_and(|c| c == ns_seg))
        })
        .unwrap_or(max);

    let mut result: ConstName = namespace[..intersection]
        .iter()
        .map(|s| Some(s.clone()))
        .collect();
    result.extend(const_name.iter().cloned());
    result
}

impl DescribedClassVisitor<'_> {
    /// Set the described class from a constant node argument to `describe`.
    fn set_described_class(&mut self, node: &ruby_prism::Node<'_>) -> bool {
        if let Some(const_name) = extract_const_name(node) {
            let full_name = collapse_namespace(&self.namespace, &const_name);
            let loc = node.location();
            let source_text = self.source.as_bytes()[loc.start_offset()..loc.end_offset()].to_vec();
            self.described_full_name = Some(full_name);
            self.described_class_source = Some(source_text);
            true
        } else {
            false
        }
    }

    /// Check if this call is a top-level describe (receiver-less or RSpec.describe)
    fn is_top_level_describe(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name().as_slice();
        if name != b"describe" {
            return false;
        }
        if let Some(recv) = call.receiver() {
            if let Some(cr) = recv.as_constant_read_node() {
                return cr.name().as_slice() == b"RSpec";
            }
            if let Some(cp) = recv.as_constant_path_node() {
                return cp.name().is_some_and(|n| n.as_slice() == b"RSpec")
                    && cp.parent().is_none();
            }
            false
        } else {
            // Must be at top-level (no described_class set yet)
            self.described_full_name.is_none()
        }
    }

    fn is_scope_change(call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name().as_slice();
        if let Some(recv) = call.receiver() {
            if let Some(cr) = recv.as_constant_read_node() {
                let class_name = cr.name().as_slice();
                if (class_name == b"Class"
                    || class_name == b"Module"
                    || class_name == b"Struct"
                    || class_name == b"Data")
                    && (name == b"new" || name == b"define")
                {
                    return true;
                }
            }
        }
        if name.ends_with(b"_eval") || name.ends_with(b"_exec") {
            return true;
        }
        false
    }

    /// Check if a resolved const name matches the described class.
    fn is_offensive_resolved(&self, const_name: &ConstName) -> bool {
        let described = match &self.described_full_name {
            Some(d) => d,
            None => return false,
        };

        let full_name = collapse_namespace(&self.namespace, const_name);
        &full_name == described
    }

    /// Extract const name from a ConstantReadNode and check if offensive.
    fn is_offensive_const_read(&self, node: &ruby_prism::ConstantReadNode<'_>) -> bool {
        let const_name = vec![Some(node.name().as_slice().to_vec())];
        self.is_offensive_resolved(&const_name)
    }

    /// Extract const name from a ConstantPathNode and check if offensive.
    fn is_offensive_const_path(&self, node: &ruby_prism::ConstantPathNode<'_>) -> bool {
        if let Some(const_name) = extract_const_name_from_path(node) {
            self.is_offensive_resolved(&const_name)
        } else {
            false
        }
    }

    /// Check if a ConstantPathNode contains `described_class` as a receiver.
    /// E.g., `described_class::CONSTANT` — we should not flag the constant path.
    fn contains_described_class(node: &ruby_prism::ConstantPathNode<'_>) -> bool {
        if let Some(parent) = node.parent() {
            if let Some(call) = parent.as_call_node() {
                if call.name().as_slice() == b"described_class"
                    && call.receiver().is_none()
                    && call.arguments().is_none()
                {
                    return true;
                }
            }
            if let Some(cp) = parent.as_constant_path_node() {
                return Self::contains_described_class(&cp);
            }
        }
        false
    }

    fn report_offense(&mut self, start_offset: usize, source_text: &[u8]) {
        let (line, col) = self.source.offset_to_line_col(start_offset);
        let described_source = self
            .described_class_source
            .as_ref()
            .and_then(|s| std::str::from_utf8(s).ok())
            .unwrap_or("?");
        let ref_text = std::str::from_utf8(source_text).unwrap_or("?");

        if self.enforced_style == "described_class" {
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                col,
                format!("Use `described_class` instead of `{}`.", ref_text),
            ));
        } else {
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                col,
                format!("Use `{}` instead of `described_class`.", described_source),
            ));
        }
    }
}

/// Extract the namespace segments from a module or class constant path.
/// `module Foo` => `["Foo"]`
/// `class Foo::Bar` => `["Foo", "Bar"]`
fn extract_module_name_segments_from_node(node: &ruby_prism::Node<'_>) -> Vec<Vec<u8>> {
    if let Some(cr) = node.as_constant_read_node() {
        return vec![cr.name().as_slice().to_vec()];
    }
    if let Some(cp) = node.as_constant_path_node() {
        let mut segments = Vec::new();
        collect_path_segments(&cp, &mut segments);
        return segments;
    }
    Vec::new()
}

fn collect_path_segments(node: &ruby_prism::ConstantPathNode<'_>, segments: &mut Vec<Vec<u8>>) {
    if let Some(parent) = node.parent() {
        if let Some(cr) = parent.as_constant_read_node() {
            segments.push(cr.name().as_slice().to_vec());
        } else if let Some(ref cp) = parent.as_constant_path_node() {
            collect_path_segments(cp, segments);
        }
    }
    if let Some(name) = node.name() {
        segments.push(name.as_slice().to_vec());
    }
}

impl<'pr> Visit<'pr> for DescribedClassVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();

        // Handle top-level describe with a class argument
        if self.is_top_level_describe(node) {
            if let Some(args) = node.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    let old_full = self.described_full_name.take();
                    let old_source = self.described_class_source.take();
                    if self.set_described_class(&arg_list[0]) {
                        if let Some(block) = node.block() {
                            if let Some(bn) = block.as_block_node() {
                                // Visit only the block body, not the arguments/receiver
                                // (the class name in describe argument is not an offense)
                                if let Some(body) = bn.body() {
                                    self.visit(&body);
                                }
                            }
                        }
                        self.described_full_name = old_full;
                        self.described_class_source = old_source;
                        return;
                    }
                    self.described_full_name = old_full;
                    self.described_class_source = old_source;
                }
            }
            // No class arg — just visit normally
            ruby_prism::visit_call_node(self, node);
            return;
        }

        // Handle nested describe with class arg — change described_class.
        // Only `describe` sets described_class, not `context`.
        if name == b"describe" && self.described_full_name.is_some() {
            if let Some(args) = node.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    let old_full = self.described_full_name.take();
                    let old_source = self.described_class_source.take();
                    if self.set_described_class(&arg_list[0]) {
                        if let Some(block) = node.block() {
                            if let Some(bn) = block.as_block_node() {
                                if let Some(body) = bn.body() {
                                    self.visit(&body);
                                }
                            }
                        }
                        self.described_full_name = old_full;
                        self.described_class_source = old_source;
                        return;
                    }
                    self.described_full_name = old_full;
                    self.described_class_source = old_source;
                }
            }
        }

        // Scope changes: don't recurse into Class.new, class_eval, etc.
        if Self::is_scope_change(node) {
            let was = self.in_scope_change;
            self.in_scope_change = true;
            ruby_prism::visit_call_node(self, node);
            self.in_scope_change = was;
            return;
        }

        // SkipBlocks: when true, don't recurse into arbitrary blocks
        if self.skip_blocks && node.block().is_some() && self.described_full_name.is_some() {
            let skip = name != b"it"
                && name != b"specify"
                && name != b"before"
                && name != b"after"
                && name != b"around"
                && name != b"let"
                && name != b"let!"
                && name != b"subject"
                && name != b"describe"
                && name != b"context";
            if skip {
                return;
            }
        }

        // "explicit" style: check for `described_class` calls
        if self.enforced_style == "explicit"
            && name == b"described_class"
            && node.receiver().is_none()
            && node.arguments().is_none()
            && self.described_full_name.is_some()
        {
            let loc = node.location();
            let source_text = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
            self.report_offense(loc.start_offset(), source_text);
        }

        // Default traversal — visits receiver, arguments, and block naturally.
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'pr>) {
        if self.in_scope_change || self.described_full_name.is_none() {
            return;
        }

        if self.enforced_style == "described_class" && self.is_offensive_const_read(node) {
            let loc = node.location();
            let source_text = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
            self.report_offense(loc.start_offset(), source_text);
        }
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'pr>) {
        if self.in_scope_change || self.described_full_name.is_none() {
            return;
        }

        if self.enforced_style == "described_class" {
            // Skip if contains described_class (e.g., described_class::CONSTANT)
            if Self::contains_described_class(node) {
                return;
            }

            // Check if the full constant path matches the described class
            if self.is_offensive_const_path(node) {
                let loc = node.location();
                let source_text = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
                self.report_offense(loc.start_offset(), source_text);
                // Don't recurse — the constant path matched as a whole
                return;
            }

            // OnlyStaticConstants: true — don't recurse into children of constant paths.
            // This prevents flagging `MyClass` in `MyClass::FOO` or `MyClass::Subclass`.
            // OnlyStaticConstants: false — recurse to check the parent part
            // (e.g., flag `MyClass` in `MyClass::FOO`).
            if self.only_static_constants {
                return;
            }
        }

        ruby_prism::visit_constant_path_node(self, node);
    }

    // Don't descend into class/module/def definitions when inside a describe block
    // (they change scope). But when outside a describe block, recurse to find
    // describe blocks nested inside module/class wrappers, tracking the namespace.
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if self.described_full_name.is_some() {
            return; // Inside describe: class is a scope change
        }
        // Outside describe: track namespace and recurse
        let segments = extract_module_name_segments_from_node(&node.constant_path());
        let added = segments.len();
        self.namespace.extend(segments);
        ruby_prism::visit_class_node(self, node);
        self.namespace.truncate(self.namespace.len() - added);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if self.described_full_name.is_some() {
            return; // Inside describe: module is a scope change
        }
        // Outside describe: track namespace and recurse
        let segments = extract_module_name_segments_from_node(&node.constant_path());
        let added = segments.len();
        self.namespace.extend(segments);
        ruby_prism::visit_module_node(self, node);
        self.namespace.truncate(self.namespace.len() - added);
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(DescribedClass, "cops/rspec/described_class");

    #[test]
    fn explicit_style_flags_described_class() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("explicit".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe MyClass do\n  it { described_class.new }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&DescribedClass, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("MyClass"));
    }

    #[test]
    fn only_static_true_flags_constant_refs() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("OnlyStaticConstants".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = b"describe MyClass do\n  it { MyClass.new }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&DescribedClass, source, config);
        assert_eq!(
            diags.len(),
            1,
            "OnlyStaticConstants: true should flag static constant refs"
        );
    }

    #[test]
    fn skip_blocks_skips_arbitrary_blocks() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("SkipBlocks".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source =
            b"describe MyClass do\n  shared_examples 'x' do\n    MyClass.new\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&DescribedClass, source, config);
        assert!(diags.is_empty(), "SkipBlocks should skip arbitrary blocks");
    }

    #[test]
    fn deeply_nested_class_reference() {
        let source = b"RSpec.describe ProblemMerge do\n  describe '#initialize' do\n    it 'creates' do\n      ProblemMerge.new(problem)\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag ProblemMerge reference in deeply nested it block"
        );
    }

    #[test]
    fn class_reference_in_let_block() {
        let source = b"RSpec.describe OutdatedProblemClearer do\n  let(:clearer) do\n    OutdatedProblemClearer.new\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag class reference inside let block"
        );
    }

    #[test]
    fn namespace_mismatch_bare_class_not_flagged() {
        let source = b"describe MyNamespace::MyClass do\n  subject { MyClass }\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            0,
            "Bare MyClass should not match MyNamespace::MyClass"
        );
    }

    #[test]
    fn namespace_mismatch_rooted_not_flagged() {
        let source = b"describe MyNamespace::MyClass do\n  subject { ::MyClass }\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            0,
            "Rooted ::MyClass should not match MyNamespace::MyClass"
        );
    }

    #[test]
    fn fully_qualified_described_class_flagged() {
        let source = b"describe MyNamespace::MyClass do\n  subject { MyNamespace::MyClass }\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            1,
            "MyNamespace::MyClass should match described class"
        );
    }

    #[test]
    fn module_qualified_described_class_flagged() {
        let source =
            b"module MyNamespace\n  describe MyClass do\n    subject { MyNamespace::MyClass }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            1,
            "MyNamespace::MyClass should match describe MyClass in module MyNamespace"
        );
    }

    #[test]
    fn only_static_constants_true_skips_constant_path() {
        let source = b"describe MyClass do\n  subject { MyClass::FOO }\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            0,
            "OnlyStaticConstants: true (default) should skip MyClass::FOO"
        );
    }

    #[test]
    fn only_static_constants_false_flags_constant_path() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("OnlyStaticConstants".into(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let source = b"describe MyClass do\n  subject { MyClass::FOO }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&DescribedClass, source, config);
        assert_eq!(
            diags.len(),
            1,
            "OnlyStaticConstants: false should flag MyClass in MyClass::FOO"
        );
    }

    #[test]
    fn include_flagged_in_described_class_style() {
        let source = b"describe MyClass do\n  include MyClass\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            1,
            "include MyClass should be flagged in described_class style"
        );
    }

    #[test]
    fn deeply_nested_namespace_resolution() {
        let source = b"module A\n  class B::C\n    module D\n      describe E do\n        subject { A::B::C::D::E }\n      end\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            1,
            "A::B::C::D::E should match E in nested namespace"
        );
    }

    #[test]
    fn innermost_describe_sets_described_class() {
        let source = b"describe MyClass do\n  describe MyClass::Foo do\n    subject { MyClass::Foo }\n    let(:foo) { MyClass }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag MyClass::Foo but not MyClass in inner describe"
        );
        assert!(
            diags[0].message.contains("MyClass::Foo"),
            "Offense should be about MyClass::Foo"
        );
    }

    #[test]
    fn collapse_namespace_basic() {
        // collapse_namespace([], [C]) => [C]
        let result = collapse_namespace(&[], &vec![Some(b"C".to_vec())]);
        assert_eq!(result, vec![Some(b"C".to_vec())]);

        // collapse_namespace([A, B], [C]) => [A, B, C]
        let result =
            collapse_namespace(&[b"A".to_vec(), b"B".to_vec()], &vec![Some(b"C".to_vec())]);
        assert_eq!(
            result,
            vec![
                Some(b"A".to_vec()),
                Some(b"B".to_vec()),
                Some(b"C".to_vec())
            ]
        );

        // collapse_namespace([A, B], [B, C]) => [A, B, C]
        let result = collapse_namespace(
            &[b"A".to_vec(), b"B".to_vec()],
            &vec![Some(b"B".to_vec()), Some(b"C".to_vec())],
        );
        assert_eq!(
            result,
            vec![
                Some(b"A".to_vec()),
                Some(b"B".to_vec()),
                Some(b"C".to_vec())
            ]
        );

        // collapse_namespace([A, B], [None, C]) => [None, C] (rooted)
        let result = collapse_namespace(
            &[b"A".to_vec(), b"B".to_vec()],
            &vec![None, Some(b"C".to_vec())],
        );
        assert_eq!(result, vec![None, Some(b"C".to_vec())]);
    }

    #[test]
    fn describe_without_class_no_offense() {
        let source = b"describe do\n  before do\n    MyClass.new\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            0,
            "describe without class arg should not set described_class"
        );
    }

    #[test]
    fn unrelated_namespace_not_flagged() {
        let source = b"module UnrelatedNamespace\n  describe MyClass do\n    subject { MyNamespace::MyClass }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&DescribedClass, source);
        assert_eq!(
            diags.len(),
            0,
            "MyNamespace::MyClass should not match describe MyClass in module UnrelatedNamespace"
        );
    }
}
