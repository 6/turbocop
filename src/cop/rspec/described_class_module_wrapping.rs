use ruby_prism::Visit;

use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, MODULE_NODE, STATEMENTS_NODE,
};
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/DescribedClassModuleWrapping: Avoid opening modules and defining specs within them.
///
/// Corpus FN=7 root cause: `contains_rspec_describe` only recursed into nested
/// `ModuleNode`s but not `ClassNode`s. The VCR corpus pattern `module VCR; class Cassette;
/// ::RSpec.describe ...` was missed because the class wrapper was not traversed.
/// Fix: refactored to a deep recursive search through both module and class bodies.
/// `::RSpec.describe` (ConstantPathNode with parent=None) was already handled.
///
/// Corpus FN=2 root cause: deep search only recursed into `ModuleNode` and
/// `ClassNode` but not other wrapper types like `SingletonClassNode` (`class << self`)
/// or `DefNode` (method definitions). The capybara pattern has `RSpec.describe` nested
/// inside `class << self` → `def specs` which was missed.
/// Fix: use `ruby_prism::Visit` for unrestricted deep descendant search, matching
/// RuboCop's `def_node_search` behavior.
///
/// Corpus FP=2 root cause: `RSpecDescribeFinder` matched any `RSpec.describe` call
/// regardless of whether it had an actual inline block. RuboCop's pattern
/// `(block (send (const nil? :RSpec) :describe ...) ...)` requires a real block node.
/// Calls like `RSpec.describe("example").metadata` (no block) and
/// `RSpec.describe(*args, &blk)` (block argument, not inline block) were false positives.
/// Fix: check `node.block().and_then(|b| b.as_block_node()).is_some()` before matching.
pub struct DescribedClassModuleWrapping;

impl Cop for DescribedClassModuleWrapping {
    fn name(&self) -> &'static str {
        "RSpec/DescribedClassModuleWrapping"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            MODULE_NODE,
            STATEMENTS_NODE,
        ]
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
        let module_node = match node.as_module_node() {
            Some(m) => m,
            None => return,
        };

        let loc = module_node.location();
        let (line, col) = source.offset_to_line_col(loc.start_offset());

        // Check if this module contains an RSpec.describe block (anywhere nested)
        if contains_rspec_describe(module_node) {
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Avoid opening modules and defining specs within them.".to_string(),
            ));
        }
    }
}

fn contains_rspec_describe(module_node: ruby_prism::ModuleNode<'_>) -> bool {
    let body = match module_node.body() {
        Some(b) => b,
        None => return false,
    };
    let mut finder = RSpecDescribeFinder { found: false };
    finder.visit(&body);
    finder.found
}

/// Visitor that does an unrestricted deep search for `RSpec.describe` calls,
/// matching RuboCop's `def_node_search` which traverses ALL descendants.
struct RSpecDescribeFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for RSpecDescribeFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if !self.found {
            let name = node.name().as_slice();
            if name == b"describe" {
                // Only match calls with an actual inline block (do...end or {}).
                // RuboCop's pattern uses `(block (send ...))` which requires
                // a real block node, not just a block argument (&blk) or bare call.
                let has_block = node.block().and_then(|b| b.as_block_node()).is_some();
                if has_block {
                    if let Some(recv) = node.receiver() {
                        if let Some(cr) = recv.as_constant_read_node() {
                            if cr.name().as_slice() == b"RSpec" {
                                self.found = true;
                                return;
                            }
                        }
                        if let Some(cp) = recv.as_constant_path_node() {
                            if cp.name().is_some_and(|n| n.as_slice() == b"RSpec")
                                && cp.parent().is_none()
                            {
                                self.found = true;
                                return;
                            }
                        }
                    }
                }
            }
            // Continue visiting children
            ruby_prism::visit_call_node(self, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        DescribedClassModuleWrapping,
        "cops/rspec/described_class_module_wrapping"
    );
}
