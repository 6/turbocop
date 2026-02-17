use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RootPathnameMethods;

const FILE_METHODS: &[&[u8]] = &[
    b"read", b"write", b"binread", b"binwrite", b"readlines",
    b"exist?", b"exists?", b"directory?", b"file?",
    b"empty?", b"size", b"delete", b"unlink",
    b"open", b"expand_path", b"realpath", b"realdirpath",
    b"basename", b"dirname", b"extname", b"join",
    b"stat", b"lstat", b"ftype", b"atime", b"ctime", b"mtime",
    b"readable?", b"writable?", b"executable?",
    b"symlink?", b"pipe?", b"socket?",
    b"zero?", b"size?", b"owned?", b"grpowned?",
    b"chmod", b"chown", b"truncate", b"rename", b"split",
    b"fnmatch", b"fnmatch?",
    b"blockdev?", b"chardev?", b"setuid?", b"setgid?", b"sticky?",
    b"readable_real?", b"writable_real?", b"executable_real?",
    b"world_readable?", b"world_writable?",
    b"readlink", b"sysopen", b"birthtime",
    b"lchmod", b"lchown", b"utime",
];

const DIR_METHODS: &[&[u8]] = &[
    b"glob", b"[]", b"exist?", b"exists?", b"mkdir", b"rmdir",
    b"children", b"each_child", b"entries", b"empty?",
    b"open", b"delete", b"unlink",
];

const FILE_TEST_METHODS: &[&[u8]] = &[
    b"blockdev?", b"chardev?", b"directory?", b"empty?",
    b"executable?", b"executable_real?", b"exist?", b"file?",
    b"grpowned?", b"owned?", b"pipe?", b"readable?",
    b"readable_real?", b"setgid?", b"setuid?", b"size", b"size?",
    b"socket?", b"sticky?", b"symlink?",
    b"world_readable?", b"world_writable?",
    b"writable?", b"writable_real?", b"zero?",
];

const FILE_UTILS_METHODS: &[&[u8]] = &[
    b"chmod", b"chown", b"mkdir", b"mkpath", b"rmdir", b"rmtree",
];

impl Cop for RootPathnameMethods {
    fn name(&self) -> &'static str {
        "Rails/RootPathnameMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        let method_name = call.name().as_slice();

        // Receiver must be a known constant (File, Dir, FileTest, FileUtils, IO)
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_name = util::constant_name(&recv);
        let is_relevant = match recv_name {
            Some(b"File") | Some(b"IO") => FILE_METHODS.contains(&method_name),
            Some(b"Dir") => DIR_METHODS.contains(&method_name),
            Some(b"FileTest") => FILE_TEST_METHODS.contains(&method_name),
            Some(b"FileUtils") => FILE_UTILS_METHODS.contains(&method_name),
            _ => false,
        };

        if !is_relevant {
            return Vec::new();
        }

        // First argument should be a Rails.root pathname:
        // Either `Rails.root.join(...)` or `Rails.root` directly
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];

        // Check if first arg is Rails.root directly
        if is_rails_root_node(first_arg) {
            let method_str = std::str::from_utf8(method_name).unwrap_or("method");
            let recv_str = std::str::from_utf8(recv_name.unwrap_or(b"File")).unwrap_or("File");
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("`Rails.root` is a `Pathname`, so you can use `Rails.root.{method_str}` instead of `{recv_str}.{method_str}(Rails.root, ...)`.",),
            )];
        }

        // Check if first arg is Rails.root.join(...)
        if let Some(arg_call) = first_arg.as_call_node() {
            if arg_call.name().as_slice() == b"join" && is_rails_root(arg_call.receiver()) {
                let method_str = std::str::from_utf8(method_name).unwrap_or("method");
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("`Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).{method_str}` instead.",),
                )];
            }
        }

        Vec::new()
    }
}

/// Check if a node is `Rails.root`
fn is_rails_root(node: Option<ruby_prism::Node<'_>>) -> bool {
    let node = match node {
        Some(n) => n,
        None => return false,
    };
    is_rails_root_node(&node)
}

fn is_rails_root_node(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    if call.name().as_slice() != b"root" {
        return false;
    }
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };
    util::constant_name(&recv) == Some(b"Rails")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RootPathnameMethods, "cops/rails/root_pathname_methods");
}
