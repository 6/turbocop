use crate::cop::{CodeMap, Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/StringHashKeys checks for the use of strings as keys in hashes.
///
/// ## Investigation findings (2026-03-13)
///
/// Root cause of 239 FPs: RuboCop has a `receive_environments_method?` matcher
/// that exempts string hash keys when the hash is passed to methods that
/// commonly use string keys for environment variables or replacement mappings:
/// - `IO.popen({"FOO" => "bar"}, ...)`
/// - `Open3.capture2/capture2e/capture3/popen2/popen2e/popen3({"FOO" => "bar"}, ...)`
/// - `Open3.pipeline/pipeline_r/pipeline_rw/pipeline_start/pipeline_w([{"FOO" => "bar"}, ...], ...)`
/// - `Kernel.spawn/system({"FOO" => "bar"}, ...)` (including bare `spawn`/`system`)
/// - `str.gsub/gsub!(pattern, {"old" => "new"})`
///
/// Fix: Converted from `check_node` to `check_source` with a visitor that
/// tracks whether we're inside an exempted method call's arguments.
///
/// ## Investigation findings (2026-03-15)
///
/// Root cause of 52 FNs: The env method exemption was too broad — it
/// exempted ALL hashes in the entire subtree of a `gsub`/`popen`/`spawn`
/// call. RuboCop's `^^` pattern only exempts hashes that are DIRECT
/// arguments (pair -> hash -> call). Hashes in the receiver chain (e.g.,
/// `{...}.to_json.gsub(...)`) or inside arrays (e.g.,
/// `IO.popen([{"FOO" => "bar"}, ...])`) are NOT exempt.
/// For `Open3.pipeline*`, RuboCop uses `^^^` (great-grandparent), which
/// allows one extra nesting level (pair -> hash -> array -> call).
///
/// Fix: Replaced depth-based exemption with a set of exempt hash byte
/// offsets. When visiting an env method call, only the direct hash/keyword
/// hash arguments (or for pipeline methods, hashes inside direct array
/// arguments) are added to the exempt set.
///
/// Root cause of 62 FPs: Heredoc strings used as hash keys. In the Parser
/// gem, heredocs are `dstr` nodes (not `str`), so RuboCop's
/// `(pair (str _) _)` matcher skips them. In Prism, heredocs without
/// interpolation are `StringNode` with `opening_loc` starting with `<<`.
///
/// Fix: Skip `StringNode` keys whose `opening_loc` starts with `<<`.
///
/// ## Investigation findings (2026-03-15, round 3)
///
/// Root cause of remaining 60 FPs: String keys with invalid UTF-8 encoding.
/// RuboCop checks `key_content.valid_encoding?` and skips strings whose
/// unescaped content is not valid in the file's encoding (typically UTF-8).
/// Examples: `"\x80"`, `"\xC0"`, `"\xFF"`, `"\251"` — these escape sequences
/// produce single bytes that are not valid UTF-8. Found in rails
/// (multibyte_chars_test.rb, inflector_test_cases.rb), puppet (pson_spec.rb,
/// evaluating_parser_spec.rb), rack, jruby, and others.
///
/// Fix: Check `std::str::from_utf8(str_node.unescaped())` and skip keys
/// where the unescaped content is not valid UTF-8.
///
/// ## Investigation findings (2026-03-17)
///
/// Root cause of 28 FPs: Multi-line regular string literals used as hash
/// keys. In the Parser gem, strings spanning multiple source lines are
/// parsed as `dstr` (dynamic string) nodes, even without interpolation.
/// RuboCop's `(pair (str _) _)` matcher only matches `str` nodes, so
/// multi-line string keys are automatically skipped. In Prism, multi-line
/// strings without interpolation are still `StringNode`.
///
/// Fix: Compare start_line and end_line of the string node's location.
/// Skip `StringNode` keys where end_line > start_line.
pub struct StringHashKeys;

impl Cop for StringHashKeys {
    fn name(&self) -> &'static str {
        "Style/StringHashKeys"
    }

    fn default_enabled(&self) -> bool {
        false
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
        let mut visitor = StringHashKeysVisitor {
            source,
            cop: self,
            diagnostics: Vec::new(),
            exempt_hash_offsets: std::collections::HashSet::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct StringHashKeysVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a StringHashKeys,
    diagnostics: Vec<Diagnostic>,
    /// Set of byte offsets for hash nodes that are exempt (direct args of env methods).
    exempt_hash_offsets: std::collections::HashSet<usize>,
}

/// Exemption depth for env method calls.
/// `Direct` means pair -> hash -> call (RuboCop's `^^` pattern).
/// `Nested` means pair -> hash -> array -> call (RuboCop's `^^^` pattern, for pipeline methods).
#[derive(Clone, Copy)]
enum EnvMethodKind {
    /// Hash must be a direct argument: `^^(send ...)`
    Direct,
    /// Hash can be inside an array argument: `^^^(send ...)`
    Nested,
}

/// Check if a call matches one of the environment-method patterns that RuboCop exempts.
/// Returns the exemption kind if matched.
fn env_method_kind(call: &ruby_prism::CallNode<'_>) -> Option<EnvMethodKind> {
    let method = call.name();
    let method_name = method.as_slice();

    match call.receiver() {
        Some(ref receiver) => {
            // IO.popen — ^^
            if method_name == b"popen" && is_const(receiver, b"IO") {
                return Some(EnvMethodKind::Direct);
            }
            if is_const(receiver, b"Open3") {
                // Open3.capture2/capture2e/capture3/popen2/popen2e/popen3 — ^^
                if matches!(
                    method_name,
                    b"capture2" | b"capture2e" | b"capture3" | b"popen2" | b"popen2e" | b"popen3"
                ) {
                    return Some(EnvMethodKind::Direct);
                }
                // Open3.pipeline* — ^^^
                if matches!(
                    method_name,
                    b"pipeline"
                        | b"pipeline_r"
                        | b"pipeline_rw"
                        | b"pipeline_start"
                        | b"pipeline_w"
                ) {
                    return Some(EnvMethodKind::Nested);
                }
            }
            // Kernel.spawn / Kernel.system — ^^
            if is_const(receiver, b"Kernel") && matches!(method_name, b"spawn" | b"system") {
                return Some(EnvMethodKind::Direct);
            }
            // anything.gsub / anything.gsub! — ^^
            if matches!(method_name, b"gsub" | b"gsub!") {
                return Some(EnvMethodKind::Direct);
            }
            None
        }
        None => {
            // Bare spawn/system (implicit Kernel receiver) — ^^
            if matches!(method_name, b"spawn" | b"system") {
                Some(EnvMethodKind::Direct)
            } else {
                None
            }
        }
    }
}

/// Check if a node is a constant read (simple or path) with the given name.
fn is_const(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        return c.name().as_slice() == name;
    }
    if let Some(cp) = node.as_constant_path_node() {
        // ::IO or just IO — parent is nil (cbase) or absent
        if cp.parent().is_none()
            || cp.parent().is_some_and(|p| {
                p.as_constant_path_node().is_none() && p.as_constant_read_node().is_none()
            })
        {
            return cp.name().is_some_and(|n| n.as_slice() == name);
        }
    }
    false
}

impl StringHashKeysVisitor<'_> {
    fn check_hash_elements<'pr, I>(&mut self, elements: I, hash_offset: usize)
    where
        I: Iterator<Item = ruby_prism::Node<'pr>>,
    {
        if self.exempt_hash_offsets.contains(&hash_offset) {
            return;
        }
        for element in elements {
            if let Some(assoc) = element.as_assoc_node() {
                let key = assoc.key();
                if let Some(str_node) = key.as_string_node() {
                    // Skip heredoc keys — in Parser gem they are `dstr` not `str`,
                    // so RuboCop's `(pair (str _) _)` matcher doesn't match them.
                    if str_node
                        .opening_loc()
                        .is_some_and(|o| o.as_slice().starts_with(b"<<"))
                    {
                        continue;
                    }
                    // Skip multi-line string keys — in Parser gem, strings
                    // spanning multiple source lines are `dstr` (not `str`),
                    // so RuboCop's `(pair (str _) _)` matcher skips them.
                    let str_loc = str_node.location();
                    let (start_line, _) = self.source.offset_to_line_col(str_loc.start_offset());
                    let (end_line, _) = self
                        .source
                        .offset_to_line_col(str_loc.end_offset().saturating_sub(1));
                    if end_line > start_line {
                        continue;
                    }
                    // Skip strings with invalid encoding — RuboCop checks
                    // `key_content.valid_encoding?` and skips them. Strings with
                    // escape sequences like \x80, \xC0, \251 produce bytes that
                    // are not valid UTF-8.
                    if std::str::from_utf8(str_node.unescaped()).is_err() {
                        continue;
                    }
                    let loc = key.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Prefer symbols instead of strings as hash keys.".to_string(),
                    ));
                }
            }
        }
    }

    /// Collect byte offsets of hash/keyword-hash nodes that are direct arguments
    /// of an env method call, so they can be exempted from the string key check.
    fn collect_exempt_hashes(&mut self, call: &ruby_prism::CallNode<'_>, kind: EnvMethodKind) {
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                match kind {
                    EnvMethodKind::Direct => {
                        // Direct hash or keyword hash argument
                        self.mark_hash_exempt(&arg);
                    }
                    EnvMethodKind::Nested => {
                        // Hash inside an array argument (one level deeper)
                        if let Some(array) = arg.as_array_node() {
                            for elem in array.elements().iter() {
                                self.mark_hash_exempt(&elem);
                            }
                        }
                    }
                }
            }
        }
    }

    fn mark_hash_exempt(&mut self, node: &ruby_prism::Node<'_>) {
        if node.as_hash_node().is_some() || node.as_keyword_hash_node().is_some() {
            self.exempt_hash_offsets
                .insert(node.location().start_offset());
        }
    }
}

impl<'pr> Visit<'pr> for StringHashKeysVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(kind) = env_method_kind(node) {
            self.collect_exempt_hashes(node, kind);
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        self.check_hash_elements(node.elements().iter(), node.location().start_offset());
        ruby_prism::visit_hash_node(self, node);
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        self.check_hash_elements(node.elements().iter(), node.location().start_offset());
        ruby_prism::visit_keyword_hash_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringHashKeys, "cops/style/string_hash_keys");
}
