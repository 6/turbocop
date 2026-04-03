use crate::cop::shared::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=1, FN=0.
///
/// Attempted fix: require `File.read` to have exactly one argument so calls
/// like `File.read(path, encoding: ...)` are ignored. That removed the known FP
/// but regressed the corpus gate from `Actual=127` to `Actual=123` against
/// `Expected=126`, introducing 3 FN.
///
/// Reverted. A correct fix needs to preserve RuboCop's positive cases for
/// `YAML.load/safe_load/parse(File.read(path), ...)` while still excluding
/// non-replaceable `File.read` variants with extra read-time options.
///
/// ## Fix (2026-03-14)
///
/// Instead of checking total arg count on `File.read`, check specifically for
/// `KeywordHashNode` in `File.read`'s arguments. Keyword args like `encoding:`
/// change reading behavior and `_file` variants don't accept them. This is more
/// targeted than the arg-count approach and preserves valid cases like
/// `YAML.load(File.read(path), permitted_classes: ...)` where the extra args
/// are on the YAML method, not on File.read.
pub struct YAMLFileRead;

/// YAML methods that should use _file variants
const YAML_METHODS: &[&[u8]] = &[b"load", b"safe_load", b"parse"];

impl Cop for YAMLFileRead {
    fn name(&self) -> &'static str {
        "Style/YAMLFileRead"
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

        let name = call.name().as_slice();
        if !YAML_METHODS.contains(&name) {
            return;
        }

        // Receiver must be YAML constant
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_yaml = if let Some(c) = receiver.as_constant_read_node() {
            c.name().as_slice() == b"YAML"
        } else if let Some(cp) = receiver.as_constant_path_node() {
            let bytes =
                &source.as_bytes()[cp.location().start_offset()..cp.location().end_offset()];
            bytes == b"YAML" || bytes == b"::YAML"
        } else {
            false
        };

        if !is_yaml {
            return;
        }

        // First argument must be File.read(...)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let is_file_read = if let Some(arg_call) = arg_list[0].as_call_node() {
            if arg_call.name().as_slice() == b"read" {
                if let Some(arg_recv) = arg_call.receiver() {
                    if let Some(c) = arg_recv.as_constant_read_node() {
                        c.name().as_slice() == b"File"
                    } else if let Some(cp) = arg_recv.as_constant_path_node() {
                        let bytes = &source.as_bytes()
                            [cp.location().start_offset()..cp.location().end_offset()];
                        bytes == b"File" || bytes == b"::File"
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if !is_file_read {
            return;
        }

        // Skip when File.read has keyword arguments (e.g., encoding: ...),
        // since those change reading behavior and _file variants don't accept them.
        if let Some(arg_call) = arg_list[0].as_call_node() {
            if let Some(file_read_args) = arg_call.arguments() {
                for file_read_arg in file_read_args.arguments().iter() {
                    if file_read_arg.as_keyword_hash_node().is_some() {
                        return;
                    }
                }
            }
        }

        // YAML.safe_load_file was introduced in Ruby 3.0;
        // skip this offense for safe_load when target Ruby version <= 2.7
        if name == b"safe_load" {
            let target_ruby = _config
                .options
                .get("TargetRubyVersion")
                .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
                .unwrap_or(3.4);
            if target_ruby <= 2.7 {
                return;
            }
        }

        let name_str = String::from_utf8_lossy(name);
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `YAML.{}_file` instead of `YAML.{}` with `File.read`.",
                name_str, name_str
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(YAMLFileRead, "cops/style/yaml_file_read");
}
