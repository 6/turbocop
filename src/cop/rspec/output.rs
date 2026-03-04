use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// RSpec/Output: flags output calls (p, puts, print, pp, $stdout.write, etc.) in specs.
///
/// Investigation: 81 FPs caused by flagging `p(...)` when used as a method argument
/// (e.g., `expect(p("abc/").normalized_pattern)`) or as a receiver of a chained call
/// (e.g., `p.trigger`). RuboCop checks `node.parent&.call_type?` and skips when the
/// output call's parent is another call node. Switched from check_node to check_source
/// with a visitor that tracks parent-is-call context. This matches the RuboCop behavior
/// at vendor/rubocop-rspec/lib/rubocop/cop/rspec/output.rb:61.
pub struct Output;

/// Output methods without a receiver (Kernel print methods)
const PRINT_METHODS: &[&[u8]] = &[b"p", b"puts", b"print", b"pp"];

/// IO write methods called on $stdout, $stderr, STDOUT, STDERR
const IO_WRITE_METHODS: &[&[u8]] = &[b"write", b"syswrite", b"binwrite"];

/// Global variable names for stdout/stderr
const GLOBAL_VARS: &[&[u8]] = &[b"$stdout", b"$stderr"];

/// Constant names for stdout/stderr
const CONST_NAMES: &[&[u8]] = &[b"STDOUT", b"STDERR"];

impl Cop for Output {
    fn name(&self) -> &'static str {
        "RSpec/Output"
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
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = OutputVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            parent_is_call: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct OutputVisitor<'a> {
    cop: &'a Output,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when the current node is a direct child (receiver/argument) of a CallNode.
    parent_is_call: bool,
}

impl<'pr> Visit<'pr> for OutputVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method = node.name().as_slice();

        // Check for output calls only when parent is NOT a call
        // (matches RuboCop: `return if node.parent&.call_type?`)
        if !self.parent_is_call {
            if PRINT_METHODS.contains(&method) && node.receiver().is_none() {
                // Skip if it has a block (p { ... } is DSL usage like phlex)
                if node.block().is_none() {
                    let mut skip = false;
                    // For `p`, skip if there are keyword args or symbol proc args
                    if method == b"p" {
                        if let Some(args) = node.arguments() {
                            for arg in args.arguments().iter() {
                                if arg.as_keyword_hash_node().is_some()
                                    || arg.as_block_argument_node().is_some()
                                {
                                    skip = true;
                                    break;
                                }
                            }
                        }
                    }
                    if !skip {
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Do not write to stdout in specs.".to_string(),
                        ));
                    }
                }
            } else if IO_WRITE_METHODS.contains(&method) {
                if let Some(recv) = node.receiver() {
                    let is_io_target = if let Some(gv) = recv.as_global_variable_read_node() {
                        GLOBAL_VARS.contains(&gv.name().as_slice())
                    } else if let Some(c) = recv.as_constant_read_node() {
                        CONST_NAMES.contains(&c.name().as_slice())
                    } else if let Some(cp) = recv.as_constant_path_node() {
                        cp.parent().is_none()
                            && cp.name().is_some()
                            && CONST_NAMES.contains(&cp.name().unwrap().as_slice())
                    } else {
                        false
                    };

                    if is_io_target && node.block().is_none() {
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Do not write to stdout in specs.".to_string(),
                        ));
                    }
                }
            }
        }

        // Visit children with parent_is_call = true for receiver/arguments,
        // but preserve default visiting for the block body
        let was = self.parent_is_call;
        self.parent_is_call = true;
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            self.visit_arguments_node(&args);
        }
        self.parent_is_call = was;

        // Visit block normally (block body is not "inside a call argument")
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Output, "cops/rspec/output");
}
