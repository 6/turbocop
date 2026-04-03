use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct GemVersion;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle reported FP=0, FN=29. All 29 FNs from gems whose ONLY version
/// constraint uses the `!=` operator, which RuboCop does not consider a version spec.
///
/// ### FN=29 → FN=3 — FIXED (commit e01710d)
///
/// Root cause: `is_version_specification()` previously used sequential
/// `trim_start_matches("!=")` which stripped `!=` from strings like `"!= 0.15.1"`,
/// leaving `" 0.15.1"` → starts with digit → treated as valid version spec. But
/// RuboCop's regex `VERSION_SPECIFICATION_REGEX = /^\s*[~<>=]*\s*[0-9.]+/` uses
/// character class `[~<>=]*` which does NOT include `!`. So `"!= 0.15.1"` fails
/// to match — RuboCop says "no version spec" and flags the gem, nitrocop didn't.
///
/// Fix: rewrote `is_version_specification()` to use a character-class approach:
/// `trim_start_matches(|c| matches!(c, '~' | '<' | '>' | '='))` — directly mirrors
/// the RuboCop regex. The `!` character is naturally excluded.
///
/// The previous fix attempt (pre-2026-03-03) that removed `.trim_start_matches("!=")`
/// from a chain of sequential strip calls caused 9 FPs. The issue was that sequential
/// stripping has order-dependent edge cases (e.g., `">=..."` vs `"=>..."`) that the
/// character-class approach avoids entirely.
///
/// All 29 FN were the same pattern — `!=`-only constraints across 6 repos:
///   - rails_admin (16 FN): `gem "cuprite", "!= 0.15.1"` and
///     `gem "rspec-expectations", "!= 3.8.3"` across Gemfile + 8 gemfiles/
///   - factory_bot_rails (9 FN): `gem "spring", "!= 2.1.1"` across 9 gemfiles/
///   - conjur (1 FN): `gem 'concurrent-ruby', '!= 1.3.5'`
///   - rails (1 FN): `gem "mdl", "!= 0.13.0"`
///   - rubocop-rspec (1 FN): `gem 'prism', '!= 1.5.0', '!= 1.5.1'`
///   - rubocop (1 FN): `gem 'memory_profiler', '!= 1.0.2'`
///
/// Note: gems with BOTH `!=` and another constraint (e.g., `gem "factory_bot",
/// ">= 4.2", "!= 6.4.5"`) were never FN because `includes_version_specification`
/// finds `">= 4.2"` first and returns true. Similarly, array-style constraints
/// like `gem "rubocop", ["~> 1.20", "!= 1.22.2"]` are not affected because
/// `includes_version_specification` only checks direct StringNode arguments (not
/// strings inside ArrayNode) — matching RuboCop's `(send nil? :gem <(str ...) ...>)`
/// node pattern, which also only matches direct str args.
///
/// ### Remaining FN — expected zero after fix
///
/// The `is_version_specification()` rewrite covers all 29 corpus FN. No remaining
/// FN are attributable to parse-error file drops. Awaiting fresh corpus run to confirm.
///
/// ### Extended corpus FN=1 (2026-03-19)
///
/// Root cause: repo `openstack__puppet-swift` has a file named `.gemfile` (dotfile).
/// Rust's `Path::extension()` returns `None` for dotfiles, so `is_ruby_file()` in
/// `fs.rs` failed to recognize `.gemfile` as a Ruby file. The file was never discovered
/// during directory walking, so no cops could run on it. Fix: added dotfile extension
/// check to `is_ruby_file()` — strips leading `.` and checks against `RUBY_EXTENSIONS`.
impl Cop for GemVersion {
    fn name(&self) -> &'static str {
        "Bundler/GemVersion"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
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
        let allowed_gems = config.get_string_array("AllowedGems").unwrap_or_default();
        let enforced_style = config.get_str("EnforcedStyle", "required");

        let mut visitor = GemVersionVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            allowed_gems,
            enforced_style,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct GemVersionVisitor<'a> {
    cop: &'a GemVersion,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    allowed_gems: Vec<String>,
    enforced_style: &'a str,
}

impl<'pr> Visit<'pr> for GemVersionVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.receiver().is_some() || node.name().as_slice() != b"gem" {
            ruby_prism::visit_call_node(self, node);
            return;
        }

        let Some(gem_name) = gem_name_from_call(node) else {
            ruby_prism::visit_call_node(self, node);
            return;
        };
        if self
            .allowed_gems
            .iter()
            .any(|allowed| allowed.as_bytes() == gem_name.as_slice())
        {
            ruby_prism::visit_call_node(self, node);
            return;
        }

        let has_version_spec = includes_version_specification(node);
        let has_commit_ref = includes_commit_reference(node);
        let offense = match self.enforced_style {
            "required" => !has_version_spec && !has_commit_ref,
            "forbidden" => has_version_spec || has_commit_ref,
            _ => false,
        };

        if offense {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let message = if self.enforced_style == "forbidden" {
                "Gem version specification is forbidden."
            } else {
                "Gem version specification is required."
            };
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                message.to_string(),
            ));
        }

        ruby_prism::visit_call_node(self, node);
    }
}

fn gem_name_from_call(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let first_arg = util::first_positional_arg(call)?;
    util::string_value(&first_arg)
}

fn includes_version_specification(call: &ruby_prism::CallNode<'_>) -> bool {
    let Some(args) = call.arguments() else {
        return false;
    };

    let mut positional_index = 0usize;
    for arg in args.arguments().iter() {
        if arg.as_keyword_hash_node().is_some() {
            continue;
        }

        if positional_index == 0 {
            positional_index += 1;
            continue;
        }
        positional_index += 1;

        if let Some(s) = arg.as_string_node() {
            if is_version_specification(s.unescaped()) {
                return true;
            }
        }
    }

    false
}

fn includes_commit_reference(call: &ruby_prism::CallNode<'_>) -> bool {
    [b"branch".as_slice(), b"ref".as_slice(), b"tag".as_slice()]
        .iter()
        .any(|key| {
            util::keyword_arg_value(call, key)
                .and_then(|value| value.as_string_node())
                .is_some()
        })
}

/// Matches RuboCop's VERSION_SPECIFICATION_REGEX: /^\s*[~<>=]*\s*[0-9.]+/
fn is_version_specification(value: &[u8]) -> bool {
    let s = std::str::from_utf8(value).unwrap_or("");
    let rest = s.trim_start();
    // Skip zero or more characters from [~<>=]
    let rest = rest.trim_start_matches(['~', '<', '>', '=']);
    let rest = rest.trim_start();
    // Must have at least one [0-9.]
    rest.starts_with(|c: char| c.is_ascii_digit() || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(GemVersion, "cops/bundler/gem_version");
}
