use crate::cop::shared::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-23):
///
/// FP=18 root cause: nitrocop flagged `File.open(f).read` without checking:
/// (1) whether `.read` has arguments (should skip `File.open(f).read(100)`),
/// (2) whether the mode arg is a valid read mode (should skip 'w', 'a', etc.).
///
/// FN=112 root cause: nitrocop only matched the chain form `File.open(x).read`.
/// Vendor also matches:
/// - Block pass form: `File.open(x, &:read)` / `File.open(x, 'rb', &:read)`
/// - Block form: `File.open(x) { |f| f.read }` / `File.open(x) do |f| f.read end`
///
/// Fix: rewrite to trigger on `File.open` CallNode, validate mode arg against
/// READ_FILE_START_TO_FINISH_MODES, then check for chain `.read` (no args),
/// `&:read` block pass, or block with single `blockvar.read` body.
/// Binary modes (ending in 'b') produce "Use `File.binread`." message.
pub struct FileRead;

/// Valid modes for reading a file from start to finish.
const READ_MODES: &[&[u8]] = &[b"r", b"rt", b"rb", b"r+", b"r+t", b"r+b"];

impl FileRead {
    fn is_file_class(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(c) = node.as_constant_read_node() {
            return c.name().as_slice() == b"File";
        }
        if let Some(cp) = node.as_constant_path_node() {
            if cp.parent().is_none() {
                return cp.name().is_some_and(|n| n.as_slice() == b"File");
            }
        }
        false
    }

    /// Check if a string node contains one of the valid read modes.
    /// Returns Some(is_binary) if valid, None if not a valid read mode.
    fn is_read_mode(node: &ruby_prism::Node<'_>) -> Option<bool> {
        let s = node.as_string_node()?;
        let content = s.unescaped();
        if READ_MODES.contains(&content) {
            Some(content.ends_with(b"b"))
        } else {
            None
        }
    }

    /// Check if block argument is `&:read`.
    fn is_read_block_pass(block: &ruby_prism::Node<'_>) -> bool {
        if let Some(ba) = block.as_block_argument_node() {
            if let Some(expr) = ba.expression() {
                if let Some(sym) = expr.as_symbol_node() {
                    return sym.unescaped() == b"read";
                }
            }
        }
        false
    }

    /// Check if a block node has a single parameter and body of `param.read` (no args).
    fn is_read_block(block: &ruby_prism::Node<'_>) -> bool {
        let bn = match block.as_block_node() {
            Some(b) => b,
            None => return false,
        };

        // Must have exactly one required parameter
        let params = match bn.parameters() {
            Some(p) => p,
            None => return false,
        };
        let bp = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return false,
        };
        let inner_params = match bp.parameters() {
            Some(p) => p,
            None => return false,
        };
        let requireds: Vec<_> = inner_params.requireds().iter().collect();
        if requireds.len() != 1 {
            return false;
        }
        // Get the parameter name
        let param_name = match requireds[0].as_required_parameter_node() {
            Some(rp) => rp.name().as_slice(),
            None => return false,
        };

        // Body must be a single statement: param.read (no args)
        let body = match bn.body() {
            Some(b) => b,
            None => return false,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return false,
        };
        let stmt_list: Vec<_> = stmts.body().iter().collect();
        if stmt_list.len() != 1 {
            return false;
        }
        let read_call = match stmt_list[0].as_call_node() {
            Some(c) => c,
            None => return false,
        };
        if read_call.name().as_slice() != b"read" {
            return false;
        }
        // .read must have no arguments
        if read_call.arguments().is_some() {
            return false;
        }
        // receiver must be the block parameter
        let recv = match read_call.receiver() {
            Some(r) => r,
            None => return false,
        };
        if let Some(lvar) = recv.as_local_variable_read_node() {
            return lvar.name().as_slice() == param_name;
        }
        false
    }

    /// Analyze the File.open call and return the offense location + message if it matches.
    /// Returns (start_offset, end_offset, is_binary) if offense found.
    fn check_file_open(call: &ruby_prism::CallNode<'_>) -> Option<(usize, usize, bool)> {
        // Must be `open` method
        if call.name().as_slice() != b"open" {
            return None;
        }

        // Receiver must be File or ::File
        let file_recv = call.receiver()?;
        if !Self::is_file_class(&file_recv) {
            return None;
        }

        // Collect positional arguments (excluding block argument)
        let args: Vec<_> = call
            .arguments()
            .map(|a| a.arguments().iter().collect::<Vec<_>>())
            .unwrap_or_default();

        // Must have at least 1 arg (filename), at most 2 (filename + mode)
        if args.is_empty() || args.len() > 2 {
            return None;
        }

        // If 2 args, second must be a valid read mode string
        let mut is_binary = false;
        if args.len() == 2 {
            match Self::is_read_mode(&args[1]) {
                Some(bin) => is_binary = bin,
                None => return None, // not a valid read mode
            }
        }

        // Check for &:read block pass
        if let Some(block) = call.block() {
            if Self::is_read_block_pass(&block) {
                let start = file_recv.location().start_offset();
                let end = call.location().end_offset();
                return Some((start, end, is_binary));
            }
            if Self::is_read_block(&block) {
                let start = file_recv.location().start_offset();
                let end = block.location().end_offset();
                return Some((start, end, is_binary));
            }
            // Has a block but it's not a simple read block — skip
            return None;
        }

        // No block pass / block — must be chain form: File.open(x).read
        // We can't check the parent from here, so we return None
        // and handle chain form from the .read side.
        None
    }

    /// Check chain form: the current node is `.read` called on `File.open(...)`.
    fn check_chain_read(call: &ruby_prism::CallNode<'_>) -> Option<(usize, usize, bool)> {
        if call.name().as_slice() != b"read" {
            return None;
        }

        // .read must have no arguments
        if call.arguments().is_some() {
            return None;
        }

        let receiver = call.receiver()?;
        let open_call = receiver.as_call_node()?;

        if open_call.name().as_slice() != b"open" {
            return None;
        }

        let file_recv = open_call.receiver()?;
        if !Self::is_file_class(&file_recv) {
            return None;
        }

        // Collect positional args
        let args: Vec<_> = open_call
            .arguments()
            .map(|a| a.arguments().iter().collect::<Vec<_>>())
            .unwrap_or_default();

        if args.is_empty() || args.len() > 2 {
            return None;
        }

        let mut is_binary = false;
        if args.len() == 2 {
            match Self::is_read_mode(&args[1]) {
                Some(bin) => is_binary = bin,
                None => return None,
            }
        }

        // open_call must not have a block (otherwise it's a different pattern)
        if open_call.block().is_some() {
            return None;
        }

        let start = file_recv.location().start_offset();
        let end = call.location().end_offset();
        Some((start, end, is_binary))
    }
}

impl Cop for FileRead {
    fn name(&self) -> &'static str {
        "Style/FileRead"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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

        // Try both patterns: block/block-pass form (triggered on `open`)
        // and chain form (triggered on `read`)
        let result = Self::check_file_open(&call).or_else(|| Self::check_chain_read(&call));

        if let Some((start, _end, is_binary)) = result {
            let (line, column) = source.offset_to_line_col(start);
            let msg = if is_binary {
                "Use `File.binread`."
            } else {
                "Use `File.read`."
            };
            diagnostics.push(self.diagnostic(source, line, column, msg.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FileRead, "cops/style/file_read");
}
