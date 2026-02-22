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
        let _only_static = config.get_bool("OnlyStaticConstants", true);

        let mut visitor = DescribedClassVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            described_class_name: None,
            described_class_short: None,
            enforced_style: enforced_style.to_string(),
            skip_blocks,
            in_scope_change: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct DescribedClassVisitor<'a> {
    cop: &'a DescribedClass,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Full described class name (e.g., b"Foo::Bar")
    described_class_name: Option<Vec<u8>>,
    /// Short (last segment) of described class name (e.g., b"Bar" for Foo::Bar)
    described_class_short: Option<Vec<u8>>,
    enforced_style: String,
    skip_blocks: bool,
    in_scope_change: bool,
}

impl DescribedClassVisitor<'_> {
    fn set_described_class(&mut self, name: Vec<u8>) {
        // Extract the short name (last segment after ::)
        let short = if let Some(pos) = name.windows(2).rposition(|w| w == b"::") {
            name[pos + 2..].to_vec()
        } else {
            name.clone()
        };
        self.described_class_short = Some(short);
        self.described_class_name = Some(name);
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
                return cp.name().map_or(false, |n| n.as_slice() == b"RSpec")
                    && cp.parent().is_none();
            }
            false
        } else {
            // Must be at top-level (no described_class set yet)
            self.described_class_name.is_none()
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

    fn is_include_extend(call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name().as_slice();
        (name == b"include" || name == b"extend" || name == b"prepend")
            && call.receiver().is_none()
    }
}

impl<'pr> Visit<'pr> for DescribedClassVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();

        // Skip include/extend/prepend — class references in these are intentional
        if Self::is_include_extend(node) {
            return;
        }

        // Handle top-level describe with a class argument
        if self.is_top_level_describe(node) {
            if let Some(args) = node.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    if let Some(class_name) = extract_constant_source(self.source, &arg_list[0]) {
                        let old_name = self.described_class_name.take();
                        let old_short = self.described_class_short.take();
                        self.set_described_class(class_name);
                        if let Some(block) = node.block() {
                            if let Some(bn) = block.as_block_node() {
                                // Visit only the block body, not the arguments/receiver
                                // (the class name in describe argument is not an offense)
                                if let Some(body) = bn.body() {
                                    self.visit(&body);
                                }
                            }
                        }
                        self.described_class_name = old_name;
                        self.described_class_short = old_short;
                        return;
                    }
                }
            }
            // No class arg — just visit normally
            ruby_prism::visit_call_node(self, node);
            return;
        }

        // Handle nested describe with class arg — change described_class.
        // Only `describe` sets described_class, not `context`.
        if name == b"describe" && self.described_class_name.is_some() {
            if let Some(args) = node.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    if let Some(nested_class) = extract_constant_source(self.source, &arg_list[0]) {
                        let old_name = self.described_class_name.take();
                        let old_short = self.described_class_short.take();
                        self.set_described_class(nested_class);
                        if let Some(block) = node.block() {
                            if let Some(bn) = block.as_block_node() {
                                if let Some(body) = bn.body() {
                                    self.visit(&body);
                                }
                            }
                        }
                        self.described_class_name = old_name;
                        self.described_class_short = old_short;
                        return;
                    }
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
        if self.skip_blocks && node.block().is_some() && self.described_class_name.is_some() {
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
        {
            if let Some(class_name) = &self.described_class_name {
                let loc = node.location();
                let (line, col) = self.source.offset_to_line_col(loc.start_offset());
                let class_str = std::str::from_utf8(class_name).unwrap_or("?");
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    col,
                    format!("Use `{class_str}` instead of `described_class`."),
                ));
            }
        }

        // Default traversal — visits receiver, arguments, and block naturally.
        // visit_constant_read_node/visit_constant_path_node will flag matching constants.
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'pr>) {
        if self.in_scope_change || self.described_class_name.is_none() {
            return;
        }
        let class_name = self.described_class_name.as_ref().unwrap().clone();

        if self.enforced_style == "described_class" {
            let name = node.name().as_slice();
            if name == class_name.as_slice()
                || self
                    .described_class_short
                    .as_ref()
                    .is_some_and(|s| name == s.as_slice() && s != &class_name)
            {
                let loc = node.location();
                let (line, col) = self.source.offset_to_line_col(loc.start_offset());
                let class_str = std::str::from_utf8(&class_name).unwrap_or("?");
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    col,
                    format!("Use `described_class` instead of `{class_str}`."),
                ));
            }
        }
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'pr>) {
        if self.in_scope_change || self.described_class_name.is_none() {
            return;
        }
        let class_name = self.described_class_name.as_ref().unwrap().clone();

        if self.enforced_style == "described_class" {
            let loc = node.location();
            let bytes = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
            if bytes == class_name.as_slice() {
                let (line, col) = self.source.offset_to_line_col(loc.start_offset());
                let class_str = std::str::from_utf8(&class_name).unwrap_or("?");
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    col,
                    format!("Use `described_class` instead of `{class_str}`."),
                ));
            }
        }
        // Don't recurse into children — the constant path is handled as a whole
    }

    // Don't descend into class/module/def definitions — they change scope
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

fn extract_constant_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    if node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some() {
        let loc = node.location();
        let bytes = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        Some(bytes.to_vec())
    } else {
        None
    }
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
            options: HashMap::from([(
                "OnlyStaticConstants".into(),
                serde_yml::Value::Bool(true),
            )]),
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
            options: HashMap::from([(
                "SkipBlocks".into(),
                serde_yml::Value::Bool(true),
            )]),
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
}
