use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, GLOBAL_VARIABLE_READ_NODE, KEYWORD_HASH_NODE};

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

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, GLOBAL_VARIABLE_READ_NODE, KEYWORD_HASH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = call.name().as_slice();

        // Case 1: bare p/puts/print/pp without receiver
        if PRINT_METHODS.contains(&method) && call.receiver().is_none() {
            // Skip if it has a block (p { ... } is DSL usage like phlex)
            if call.block().is_some() {
                return Vec::new();
            }

            // For `p`, skip if there are keyword args or symbol proc args
            // (which indicates DSL usage, not printing)
            if method == b"p" {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
                    for arg in &arg_list {
                        if arg.as_keyword_hash_node().is_some() {
                            return Vec::new();
                        }
                        if arg.as_block_argument_node().is_some() {
                            return Vec::new();
                        }
                    }
                }
            }

            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Do not write to stdout in specs.".to_string(),
            )];
        }

        // Case 2: $stdout.write, $stderr.syswrite, STDOUT.write, STDERR.write, etc.
        if IO_WRITE_METHODS.contains(&method) {
            if let Some(recv) = call.receiver() {
                let is_io_target =
                    if let Some(gv) = recv.as_global_variable_read_node() {
                        GLOBAL_VARS.contains(&gv.name().as_slice())
                    } else if let Some(c) = recv.as_constant_read_node() {
                        CONST_NAMES.contains(&c.name().as_slice())
                    } else if let Some(cp) = recv.as_constant_path_node() {
                        // ::STDOUT, ::STDERR
                        cp.parent().is_none()
                            && cp.name().is_some()
                            && CONST_NAMES.contains(&cp.name().unwrap().as_slice())
                    } else {
                        false
                    };

                if is_io_target {
                    // Skip if it has a block (write { ... })
                    if call.block().is_some() {
                        return Vec::new();
                    }

                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not write to stdout in specs.".to_string(),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Output, "cops/rspec/output");
}
