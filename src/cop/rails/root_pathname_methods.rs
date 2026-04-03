use crate::cop::shared::constant_predicates;
use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-15):
///
/// **FPs fixed (4):** All 4 FPs were `File.open(Rails.root.join(...))` used as part of a larger
/// expression (argument to another method or receiver of a chain like `.read`). RuboCop skips
/// `File.open(...)` when `node.parent&.send_type?` because these patterns are handled by
/// `Style/FileRead` / `Style/FileWrite` instead. Since Prism lacks parent pointers, we detect
/// this by checking the source byte after the node's end offset (`.`, `)`, `,` indicate the
/// node is part of a larger expression).
///
/// **FNs fixed (21):** All 21 FNs used `Rails.public_path` instead of `Rails.root`. RuboCop
/// checks both `{:root :public_path}` in its `rails_root?` matcher. Added `public_path`
/// support to `rails_root_method_from_node`.
///
/// **Corpus investigation (2026-03-15, round 2):**
///
/// **FP fixed (1):** `obj.attr = File.open(Rails.root.join(...))` — setter assignment context.
/// RuboCop skips because `node.parent` is a send (`attr=`). Fixed by also checking the byte
/// before the node for `=` (but not `==`).
///
/// **FNs fixed (127):** Nearly all were `File.open(Rails.root.join(...))` inside hash literals,
/// e.g. `{io: File.open(Rails.root.join("public", "photo.png")), ...}`. The old after-node
/// heuristic checked for `,` and `)` which incorrectly skipped these (hash commas, not call
/// args). Refined to only check `.` after the node (chain receiver) and `(`/`,`/`=` before
/// the node (argument to call / setter).
///
/// **Corpus investigation (2026-03-16):**
///
/// **FNs fixed (26):** All 26 FNs were `var = File.open(Rails.root.join(...))` — plain local,
/// instance, or class variable assignments (e.g., `file = File.open(...)`,
/// `@file = File.open(...)`, `f = File.open(...)`, `@x ||= File.open(...)`). The previous
/// `=` heuristic treated ALL `=` (except `==`) as setter-method assignments and skipped them.
/// Root cause: RuboCop's `node.parent&.send_type?` check only skips when the parent is a
/// *send* node (setter method like `obj.attr=`); for local/instance variable assignments the
/// parent is `lvasgn`/`ivasgn`, not a send, so RuboCop still reports the offense. Fixed by
/// refining the `=` heuristic: only skip if the bytes before `=` show a dotted LHS
/// (e.g., `obj.attr =`). Compound operators (`||=`, `&&=`, `+=`, etc.) and simple variable
/// identifiers are no longer skipped.
///
/// **Corpus investigation (2026-03-19):**
///
/// **FPs fixed (12):** All 12 FPs were `File.exists?(Rails.root.join(...))` or similar calls
/// using `exists?` (with the trailing 's'). RuboCop's `FILE_METHODS`, `DIR_METHODS`, and
/// `FILE_TEST_METHODS` only include `exist?` (without the 's'), not the deprecated `exists?`.
/// `File.exists?` / `Dir.exists?` are deprecated Ruby methods. Removed `exists?` from
/// nitrocop's method lists to match RuboCop.
pub struct RootPathnameMethods;

const FILE_METHODS: &[&[u8]] = &[
    b"read",
    b"write",
    b"binread",
    b"binwrite",
    b"readlines",
    b"exist?",
    b"directory?",
    b"file?",
    b"empty?",
    b"size",
    b"delete",
    b"unlink",
    b"open",
    b"expand_path",
    b"realpath",
    b"realdirpath",
    b"basename",
    b"dirname",
    b"extname",
    b"join",
    b"stat",
    b"lstat",
    b"ftype",
    b"atime",
    b"ctime",
    b"mtime",
    b"readable?",
    b"writable?",
    b"executable?",
    b"symlink?",
    b"pipe?",
    b"socket?",
    b"zero?",
    b"size?",
    b"owned?",
    b"grpowned?",
    b"chmod",
    b"chown",
    b"truncate",
    b"rename",
    b"split",
    b"fnmatch",
    b"fnmatch?",
    b"blockdev?",
    b"chardev?",
    b"setuid?",
    b"setgid?",
    b"sticky?",
    b"readable_real?",
    b"writable_real?",
    b"executable_real?",
    b"world_readable?",
    b"world_writable?",
    b"readlink",
    b"sysopen",
    b"birthtime",
    b"lchmod",
    b"lchown",
    b"utime",
];

const DIR_METHODS: &[&[u8]] = &[
    b"glob",
    b"[]",
    b"exist?",
    b"mkdir",
    b"rmdir",
    b"children",
    b"each_child",
    b"entries",
    b"empty?",
    b"open",
    b"delete",
    b"unlink",
];

const FILE_TEST_METHODS: &[&[u8]] = &[
    b"blockdev?",
    b"chardev?",
    b"directory?",
    b"empty?",
    b"executable?",
    b"executable_real?",
    b"exist?",
    b"file?",
    b"grpowned?",
    b"owned?",
    b"pipe?",
    b"readable?",
    b"readable_real?",
    b"setgid?",
    b"setuid?",
    b"size",
    b"size?",
    b"socket?",
    b"sticky?",
    b"symlink?",
    b"world_readable?",
    b"world_writable?",
    b"writable?",
    b"writable_real?",
    b"zero?",
];

const FILE_UTILS_METHODS: &[&[u8]] =
    &[b"chmod", b"chown", b"mkdir", b"mkpath", b"rmdir", b"rmtree"];

impl Cop for RootPathnameMethods {
    fn name(&self) -> &'static str {
        "Rails/RootPathnameMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        // Receiver must be a known constant (File, Dir, FileTest, FileUtils, IO)
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_name = constant_predicates::constant_short_name(&recv);
        let is_relevant = match recv_name {
            Some(b"File") | Some(b"IO") => FILE_METHODS.contains(&method_name),
            Some(b"Dir") => DIR_METHODS.contains(&method_name),
            Some(b"FileTest") => FILE_TEST_METHODS.contains(&method_name),
            Some(b"FileUtils") => FILE_UTILS_METHODS.contains(&method_name),
            _ => false,
        };

        if !is_relevant {
            return;
        }

        // RuboCop skips `File.open(...)` / `IO.open(...)` when the parent is a send node.
        // This handles cases like `File.open(Rails.root.join(...)).read` (receiver of chain),
        // `YAML.safe_load(File.open(Rails.root.join(...)))` (argument to another call),
        // or `obj.attr = File.open(Rails.root.join(...))` (argument to setter method).
        // These are handled by Style/FileRead and Style/FileWrite instead.
        // Since Prism doesn't provide parent pointers, we check the source bytes around
        // the node to detect if it's part of a larger expression (i.e., has a send parent).
        if method_name == b"open" {
            let src = source.as_bytes();
            let start_offset = node.location().start_offset();
            let end_offset = node.location().end_offset();

            // Check after the node: `.` means receiver of method chain
            // e.g., File.open(Rails.root.join(...)).read
            // We intentionally do NOT check for `)` or `,` after the node, because
            // those often indicate hash entry separators (e.g., `io: File.open(...),`)
            // where the parent is a `pair` node, not a send, and RuboCop flags those.
            let after = &src[end_offset..];
            let next_meaningful = after.iter().find(|&&b| b != b' ' && b != b'\t');
            if next_meaningful == Some(&b'.') {
                return;
            }

            // Check before the node for nesting indicators
            // `(` means argument to a call: YAML.safe_load(File.open(...))
            // `,` means second+ arg: foo(x, File.open(...))
            // `=` on RHS of setter method: obj.attr = File.open(...)
            //   but NOT simple assignment (`var = File.open(...)`) — those ARE offenses.
            //   and NOT compound assignment (`||= File.open(...)`) — those ARE offenses.
            //   and NOT comparison (`== File.open(...)`) — not an assignment at all.
            if start_offset > 0 {
                let before = &src[..start_offset];
                let prev_meaningful_pos = before.iter().rposition(|&b| b != b' ' && b != b'\t');
                if let Some(pos) = prev_meaningful_pos {
                    match before[pos] {
                        b'(' | b',' => return,
                        b'=' => {
                            // Only skip for setter method assignments like `obj.attr = File.open(...)`
                            // These are identified by having a `.method` expression on the LHS.
                            // Do NOT skip for:
                            //   - `==` (comparison)
                            //   - `||=`, `&&=`, `+=`, etc. (compound assignments, LHS is a variable)
                            //   - `var = File.open(...)` (simple local/instance variable assignment)
                            let is_setter = pos > 0 && {
                                let prev = before[pos - 1];
                                // Not `==`, not a compound operator, and LHS is a method call chain
                                prev != b'='
                                    && !matches!(
                                        prev,
                                        b'|' | b'&' | b'+' | b'-' | b'*' | b'/' | b'%' | b'^'
                                    )
                                    && is_setter_method_assignment(before, pos)
                            };
                            if is_setter {
                                return;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // First argument should be a Rails.root pathname:
        // Either `Rails.root.join(...)` or `Rails.root` directly
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let first_arg = &arg_list[0];

        // Check if first arg is Rails.root or Rails.public_path directly
        if let Some(rails_label) = rails_root_method_from_node(first_arg) {
            let method_str = std::str::from_utf8(method_name).unwrap_or("method");
            let recv_str = std::str::from_utf8(recv_name.unwrap_or(b"File")).unwrap_or("File");
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("`{rails_label}` is a `Pathname`, so you can use `{rails_label}.{method_str}` instead of `{recv_str}.{method_str}({rails_label}, ...)`.",),
            ));
        }

        // Check if first arg is Rails.root.join(...) or Rails.public_path.join(...)
        if let Some(arg_call) = first_arg.as_call_node() {
            if arg_call.name().as_slice() == b"join" {
                if let Some(rails_label) = rails_root_method(arg_call.receiver()) {
                    let method_str = std::str::from_utf8(method_name).unwrap_or("method");
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("`{rails_label}` is a `Pathname`, so you can use `{rails_label}.join(...).{method_str}` instead.",),
                    ));
                }
            }
        }
    }
}

/// Determine if the `=` at `eq_pos` in `before` is a setter method assignment like
/// `obj.attr = ` rather than a simple local/instance/class variable assignment.
///
/// Returns `true` when the LHS contains a `.` before the final identifier, meaning
/// the assignment is dispatched as a send (e.g., `obj.attr=`). Returns `false` for
/// simple assignments like `var = `, `@ivar = `, `@@cvar = `, `CONST = `.
fn is_setter_method_assignment(before: &[u8], eq_pos: usize) -> bool {
    // `before[..eq_pos]` ends just before the `=`.
    // Skip trailing spaces/tabs to find the end of the LHS expression.
    let lhs = &before[..eq_pos];
    let Some(lhs_end) = lhs.iter().rposition(|&b| b != b' ' && b != b'\t') else {
        return false;
    };

    // Scan backwards over identifier characters (word chars including `?` and `!` for Ruby methods).
    let mut scan = lhs_end;
    while scan > 0 && is_ruby_ident_byte(lhs[scan]) {
        scan -= 1;
    }
    // Check if the byte immediately before the identifier is `.`
    if scan > 0 && lhs[scan] == b'.' {
        return true; // setter method: `obj.attr =`
    }
    // Handle the case where scan reached the start (e.g. the char at index 0 is ident)
    if scan == 0 && is_ruby_ident_byte(lhs[0]) {
        return false; // bare identifier at start = simple assignment
    }
    false
}

/// Returns true for bytes that can appear in a Ruby identifier/method name.
fn is_ruby_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'?' || b == b'!'
}

/// Check if a node is `Rails.root` or `Rails.public_path`, returning the method name.
fn rails_root_method(node: Option<ruby_prism::Node<'_>>) -> Option<&'static str> {
    let node = node?;
    rails_root_method_from_node(&node)
}

fn rails_root_method_from_node(node: &ruby_prism::Node<'_>) -> Option<&'static str> {
    let call = node.as_call_node()?;
    let method = call.name().as_slice();
    let label = match method {
        b"root" => "Rails.root",
        b"public_path" => "Rails.public_path",
        _ => return None,
    };
    let recv = call.receiver()?;
    if constant_predicates::constant_short_name(&recv) == Some(b"Rails") {
        Some(label)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RootPathnameMethods, "cops/rails/root_pathname_methods");
}
