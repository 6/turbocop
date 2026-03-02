use crate::cop::{Cop, CopConfig, util};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct GemVersion;

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

fn is_version_specification(value: &[u8]) -> bool {
    let s = std::str::from_utf8(value).unwrap_or("").trim();
    let stripped = s
        .trim_start_matches("~>")
        .trim_start_matches(">=")
        .trim_start_matches("<=")
        .trim_start_matches("==")
        .trim_start_matches("!=")
        .trim_start_matches('>')
        .trim_start_matches('<')
        .trim_start_matches('=')
        .trim();

    stripped
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(GemVersion, "cops/bundler/gem_version");
}
