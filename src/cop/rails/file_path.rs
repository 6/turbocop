use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, EMBEDDED_STATEMENTS_NODE,
    INTERPOLATED_STRING_NODE, LOCAL_VARIABLE_READ_NODE, STRING_NODE,
};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/FilePath cop — flags non-idiomatic file path construction with Rails.root.
///
/// ## Investigation findings (2026-03-15)
///
/// **FP root cause**: `File.join(Rails.root, ...)` was only excluding local variable
/// arguments from flagging. RuboCop's `arg.variable?` excludes ALL variable types
/// (local, instance `@`, class `@@`, global `$`). Instance variables like `@current_db`
/// in `File.join(Rails.root, "tmp", "backups", @current_db, @timestamp)` were incorrectly
/// flagged. Also missing: check for `string_contains_multiple_slashes?` in File.join args.
///
/// **FN root causes (28 FNs)**:
/// 1. `Rails.root.join` multi-arg (slashes style) missing leading-slash exclusion —
///    `Rails.root.join("app", "/models")` should not be flagged, was being flagged.
/// 2. `File.join` with array arguments (`File.join(Rails.root, ['a','b'])`,
///    `File.join(Rails.root, %w[a b])`) not detected — arrays are valid args in RuboCop.
/// 3. `"#{Rails.root.join('tmp','icon')}.png"` extension-after-join pattern not detected.
/// 4. `arguments` style `Rails.root.join('app/models')` only flagged single-arg; should
///    also flag multi-arg when any string arg contains a slash.
/// 5. String interpolation `"#{Rails.root}/path"` missing guard for colon-separated paths
///    (`"#{Rails.root}:/foo"`) and non-send embedded statements (`#{Rails.root || '.'}`).
///
/// **Fixes applied**: Added instance/class/global variable checks, array arg support,
/// leading-slash and multi-slash exclusions for both File.join and Rails.root.join,
/// extension-after-join detection in dstr, colon guard for dstr, non-send guard for dstr,
/// and multi-arg slash detection in arguments style.
///
/// ## Investigation findings (2026-03-16)
///
/// **FP root cause**: `is_rails_root` and `File.join` receiver check used
/// `util::constant_name()` which only compares the last segment of a constant path.
/// `SomeModule::Rails.root` and `SomeModule::File.join(...)` were incorrectly matched
/// as `Rails.root` and `File.join`. RuboCop's pattern `(const {nil? cbase} :Rails)`
/// requires the constant to be top-level (bare or `::` prefixed). Added
/// `is_top_level_constant()` guard to both checks.
///
/// **FN root cause**: Extension-after-Rails.root in dstr (`"#{Rails.root}.png"`) was
/// guarded by `is_rails_root_join()`, only detecting `"#{Rails.root.join(...)}.ext"`.
/// RuboCop's `check_for_extension_after_rails_root_join_in_dstr` does NOT require the
/// inner expression to be `.join()` — it checks extension for any dstr containing
/// Rails.root. Removed the `is_rails_root_join` guard.
pub struct FilePath;

/// Check if a constant node is top-level (bare `Foo` or `::Foo`), not namespaced (`A::Foo`).
/// Matches RuboCop's `(const {nil? cbase} :Name)` pattern.
fn is_top_level_constant(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_constant_read_node().is_some() {
        return true; // bare constant like `Rails` or `File`
    }
    if let Some(cp) = node.as_constant_path_node() {
        return cp.parent().is_none(); // `::Rails` or `::File` (cbase)
    }
    false
}

/// Check if a node is `Rails.root` or `::Rails.root` (not `SomeModule::Rails.root`).
fn is_rails_root(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"root" {
            if let Some(recv) = call.receiver() {
                return util::constant_name(&recv) == Some(b"Rails")
                    && is_top_level_constant(&recv);
            }
        }
    }
    false
}

/// Check if a node is `Rails.root.join(...)` (a join call on Rails.root).
fn is_rails_root_join(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"join" {
            if let Some(recv) = call.receiver() {
                return is_rails_root(&recv);
            }
        }
    }
    false
}

/// Recursively check if a node is or contains Rails.root.
fn contains_rails_root(node: &ruby_prism::Node<'_>) -> bool {
    if is_rails_root(node) {
        return true;
    }
    // Check array arguments: [Rails.root, ...]
    if let Some(arr) = node.as_array_node() {
        return arr.elements().iter().any(|e| contains_rails_root(&e));
    }
    false
}

/// Check if a node is any kind of variable (local, instance, class, global).
fn is_variable(node: &ruby_prism::Node<'_>) -> bool {
    node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
}

/// Check if a string node contains `//` (multiple slashes).
fn string_contains_multiple_slashes(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_string_node() {
        let val = s.unescaped();
        val.windows(2).any(|w| w == b"//")
    } else {
        false
    }
}

/// Check if a string node starts with `/`.
fn string_with_leading_slash(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_string_node() {
        s.unescaped().starts_with(b"/")
    } else {
        false
    }
}

/// Check if a string node contains `/`.
fn string_contains_slash(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_string_node() {
        s.unescaped().windows(1).any(|w| w == b"/")
    } else {
        false
    }
}

/// Check if a node is a constant (not Rails).
fn is_non_rails_constant(node: &ruby_prism::Node<'_>) -> bool {
    (node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some())
        && util::constant_name(node) != Some(b"Rails")
}

impl Cop for FilePath {
    fn name(&self) -> &'static str {
        "Rails/FilePath"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            EMBEDDED_STATEMENTS_NODE,
            INTERPOLATED_STRING_NODE,
            LOCAL_VARIABLE_READ_NODE,
            STRING_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "slashes");

        // Check string interpolation: "#{Rails.root}/path/to" and "#{Rails.root.join(...)}.ext"
        if let Some(istr) = node.as_interpolated_string_node() {
            self.check_dstr(source, &istr, style, diagnostics);
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"join" {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if util::constant_name(&recv) == Some(b"File") && is_top_level_constant(&recv) {
            // Pattern 1: File.join(Rails.root, ...) — receiver is File or ::File constant
            self.check_file_join(source, node, &call, style, diagnostics);
            return;
        }

        // Pattern 2: Rails.root.join('path', 'to') — receiver is Rails.root
        if !is_rails_root(&recv) {
            return;
        }

        self.check_rails_root_join(source, node, &call, style, diagnostics);
    }
}

impl FilePath {
    fn check_dstr(
        &self,
        source: &SourceFile,
        istr: &ruby_prism::InterpolatedStringNode<'_>,
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let parts: Vec<_> = istr.parts().iter().collect();

        // Find the index of the embedded node containing Rails.root
        let rails_root_index = parts.iter().position(|part| {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    return body.len() == 1 && contains_rails_root_deep(&body[0]);
                }
            }
            false
        });

        let rails_root_index = match rails_root_index {
            Some(idx) => idx,
            None => return,
        };

        // Check for colon separator after Rails.root: "#{Rails.root}:/foo/bar"
        if dstr_separated_by_colon(&parts, rails_root_index) {
            return;
        }

        // Get the embedded node's inner expression
        let embedded = parts[rails_root_index]
            .as_embedded_statements_node()
            .unwrap();
        let stmts = embedded.statements().unwrap();
        let body: Vec<_> = stmts.body().iter().collect();
        let inner_expr = &body[0];

        // Check for extension after Rails.root or Rails.root.join:
        // "#{Rails.root}.png" or "#{Rails.root.join(...)}.png"
        // RuboCop checks extension regardless of whether inner expr is .join or bare .root
        if let Some(next_part) = parts.get(rails_root_index + 1) {
            if is_extension_node(source, next_part) {
                let loc = istr.as_node().location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let msg = self.build_message(style, false);
                diagnostics.push(self.diagnostic(source, line, column, msg));
                return;
            }
        }

        // Check for slash after Rails.root: "#{Rails.root}/path"
        // The embedded expression must be a simple send (not `||`, `rescue`, etc.)
        if inner_expr.as_call_node().is_none() {
            return;
        }

        if let Some(next_part) = parts.get(rails_root_index + 1) {
            if let Some(str_part) = next_part.as_string_node() {
                if str_part.unescaped().starts_with(b"/") {
                    let loc = istr.as_node().location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let msg = self.build_message(style, false);
                    diagnostics.push(self.diagnostic(source, line, column, msg));
                }
            }
        }
    }

    fn check_file_join(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        call: &ruby_prism::CallNode<'_>,
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();

        // Check if any argument (including inside arrays) contains Rails.root
        let has_rails_root = arg_list.iter().any(|a| contains_rails_root(a));
        if !has_rails_root {
            return;
        }

        // Check that no arguments are variables, non-Rails constants, or contain multiple slashes
        // RuboCop: arguments.none? { |arg| arg.variable? || arg.const_type? || string_contains_multiple_slashes?(arg) }
        let has_invalid_arg = arg_list.iter().any(|a| {
            is_variable(a) || is_non_rails_constant(a) || string_contains_multiple_slashes(a)
        });
        if has_invalid_arg {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let msg = self.build_message(style, true);
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }

    fn check_rails_root_join(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        call: &ruby_prism::CallNode<'_>,
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();

        match style {
            "arguments" => {
                // Flag args that contain a slash (but not leading slash or multiple slashes)
                // RuboCop: valid_slash_separated_path_for_rails_root_join?
                let has_slash = arg_list.iter().any(|a| string_contains_slash(a));
                if !has_slash {
                    return;
                }
                // Skip if any arg has a leading slash or multiple slashes
                let has_excluded = arg_list
                    .iter()
                    .any(|a| string_with_leading_slash(a) || string_contains_multiple_slashes(a));
                if has_excluded {
                    return;
                }

                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let msg = self.build_message(style, false);
                diagnostics.push(self.diagnostic(source, line, column, msg));
            }
            _ => {
                // "slashes" (default): flag multi-arg join calls where all args are strings
                // RuboCop: valid_string_arguments_for_rails_root_join?
                if arg_list.len() < 2 {
                    return;
                }
                let all_strings = arg_list.iter().all(|a| a.as_string_node().is_some());
                if !all_strings {
                    return;
                }
                // Skip if any arg has a leading slash or multiple slashes
                let has_excluded = arg_list
                    .iter()
                    .any(|a| string_with_leading_slash(a) || string_contains_multiple_slashes(a));
                if has_excluded {
                    return;
                }

                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let msg = self.build_message(style, false);
                diagnostics.push(self.diagnostic(source, line, column, msg));
            }
        }
    }

    fn build_message(&self, style: &str, require_to_s: bool) -> String {
        let to_s = if require_to_s { ".to_s" } else { "" };
        if style == "arguments" {
            format!("Prefer `Rails.root.join('path', 'to'){to_s}`.")
        } else {
            format!("Prefer `Rails.root.join('path/to'){to_s}`.")
        }
    }
}

/// Recursively check if a node contains Rails.root (deep check for dstr).
fn contains_rails_root_deep(node: &ruby_prism::Node<'_>) -> bool {
    if is_rails_root(node) {
        return true;
    }
    if is_rails_root_join(node) {
        return true;
    }
    // Check call receiver chain
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            return contains_rails_root_deep(&recv);
        }
    }
    false
}

/// Check if the dstr is separated by a colon after Rails.root (e.g. "#{Rails.root}:/foo").
fn dstr_separated_by_colon(parts: &[ruby_prism::Node<'_>], rails_root_index: usize) -> bool {
    for part in &parts[rails_root_index + 1..] {
        if let Some(s) = part.as_string_node() {
            let src = s.unescaped();
            if src.starts_with(b":") {
                return true;
            }
        }
    }
    false
}

/// Check if a node is a file extension pattern (e.g. ".png", ".jpg").
fn is_extension_node(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_string_node() {
        let loc = s.location();
        let src_bytes = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        // Check source text starts with a dot followed by letters (e.g. ".png")
        if src_bytes.first() == Some(&b'.') {
            return src_bytes[1..].iter().all(|&b| b.is_ascii_alphabetic());
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FilePath, "cops/rails/file_path");

    #[test]
    fn arguments_style_flags_slash_in_single_arg() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("arguments".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"Rails.root.join('app/models')\n";
        let diags = run_cop_full_with_config(&FilePath, source, config);
        assert!(
            !diags.is_empty(),
            "arguments style should flag slash-separated path"
        );
    }

    #[test]
    fn arguments_style_allows_multi_arg() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("arguments".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"Rails.root.join('app', 'models')\n";
        assert_cop_no_offenses_full_with_config(&FilePath, source, config);
    }

    #[test]
    fn arguments_style_flags_multi_arg_with_slash() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("arguments".to_string()),
            )]),
            ..CopConfig::default()
        };
        // Multi-arg where one arg has a slash should be flagged in arguments style
        let source = b"Rails.root.join('app/models', 'user.rb')\n";
        let diags = run_cop_full_with_config(&FilePath, source, config);
        assert!(
            !diags.is_empty(),
            "arguments style should flag multi-arg with slash in arg"
        );
    }

    #[test]
    fn slashes_style_skips_leading_slash_args() {
        use crate::testutil::assert_cop_no_offenses_full;
        // Leading slash in any arg should not be flagged
        assert_cop_no_offenses_full(&FilePath, b"Rails.root.join('app', '/models')\n");
        assert_cop_no_offenses_full(&FilePath, b"Rails.root.join('/app', 'models')\n");
    }

    #[test]
    fn dstr_colon_separator_no_offense() {
        use crate::testutil::assert_cop_no_offenses_full;
        assert_cop_no_offenses_full(&FilePath, b"\"#{Rails.root}:/foo/bar\"\n");
    }

    #[test]
    fn dstr_extension_after_join() {
        use crate::testutil::run_cop_full;
        let source = b"\"#{Rails.root.join('tmp', user.id, 'icon')}.png\"\n";
        let diags = run_cop_full(&FilePath, source);
        assert!(
            !diags.is_empty(),
            "should flag extension after Rails.root.join in dstr"
        );
    }

    #[test]
    fn file_join_with_instance_var_no_offense() {
        use crate::testutil::assert_cop_no_offenses_full;
        assert_cop_no_offenses_full(&FilePath, b"File.join(Rails.root, 'app', @default_path)\n");
    }

    #[test]
    fn file_join_with_class_var_no_offense() {
        use crate::testutil::assert_cop_no_offenses_full;
        assert_cop_no_offenses_full(&FilePath, b"File.join(Rails.root, 'app', @@default_path)\n");
    }

    #[test]
    fn file_join_with_global_var_no_offense() {
        use crate::testutil::assert_cop_no_offenses_full;
        assert_cop_no_offenses_full(&FilePath, b"File.join(Rails.root, 'app', $default_path)\n");
    }

    #[test]
    fn file_join_with_array_offense() {
        use crate::testutil::run_cop_full;
        let diags = run_cop_full(&FilePath, b"File.join(Rails.root, ['app', 'models'])\n");
        assert!(
            !diags.is_empty(),
            "should flag File.join with array argument containing Rails.root"
        );
    }

    #[test]
    fn namespaced_rails_root_no_offense() {
        use crate::testutil::assert_cop_no_offenses_full;
        // SomeModule::Rails.root is NOT the same as Rails.root
        assert_cop_no_offenses_full(&FilePath, b"SomeModule::Rails.root.join('app', 'models')\n");
    }

    #[test]
    fn namespaced_file_join_no_offense() {
        use crate::testutil::assert_cop_no_offenses_full;
        // SomeModule::File.join should not be treated as File.join
        assert_cop_no_offenses_full(
            &FilePath,
            b"SomeModule::File.join(Rails.root, 'app', 'models')\n",
        );
    }

    #[test]
    fn dstr_namespaced_rails_root_no_offense() {
        use crate::testutil::assert_cop_no_offenses_full;
        assert_cop_no_offenses_full(&FilePath, b"\"#{SomeModule::Rails.root}/path\"\n");
    }

    #[test]
    fn dstr_extension_after_bare_rails_root() {
        use crate::testutil::run_cop_full;
        // "#{Rails.root}.png" should be flagged (extension after bare Rails.root)
        let diags = run_cop_full(&FilePath, b"\"#{Rails.root}.png\"\n");
        assert!(
            !diags.is_empty(),
            "should flag extension after bare Rails.root in dstr"
        );
    }
}
