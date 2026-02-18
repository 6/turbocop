use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct InferredSpecType;

/// Default directory-to-type inferences (matching RuboCop's defaults).
const DEFAULT_INFERENCES: &[(&str, &str)] = &[
    ("channels", "channel"),
    ("controllers", "controller"),
    ("features", "feature"),
    ("generator", "generator"),
    ("helpers", "helper"),
    ("jobs", "job"),
    ("mailboxes", "mailbox"),
    ("mailers", "mailer"),
    ("models", "model"),
    ("requests", "request"),
    ("integration", "request"),
    ("api", "request"),
    ("routing", "routing"),
    ("system", "system"),
    ("views", "view"),
];

/// Example group methods that can have type metadata.
const EXAMPLE_GROUPS: &[&[u8]] = &[
    b"describe",
    b"context",
    b"feature",
    b"example_group",
];

impl Cop for InferredSpecType {
    fn name(&self) -> &'static str {
        "RSpecRails/InferredSpecType"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_RAILS_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check for RSpec.describe or bare describe/context/etc.
        let is_example_group = if let Some(recv) = call.receiver() {
            crate::cop::util::constant_name(&recv).map_or(false, |n| n == b"RSpec")
                && (method_name == b"describe" || method_name == b"context")
        } else {
            EXAMPLE_GROUPS.contains(&method_name)
        };

        if !is_example_group {
            return Vec::new();
        }

        // Must have a block
        if call.block().is_none() {
            return Vec::new();
        }

        // Look for `type:` keyword argument in the arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Find a hash argument containing `type: :something`
        for arg in args.arguments().iter() {
            if let Some(diag) = self.check_hash_arg(source, &arg, config) {
                return vec![diag];
            }
        }

        Vec::new()
    }
}

impl InferredSpecType {
    fn check_hash_arg(
        &self,
        source: &SourceFile,
        arg: &ruby_prism::Node<'_>,
        config: &CopConfig,
    ) -> Option<Diagnostic> {
        if let Some(hash) = arg.as_hash_node() {
            return self.check_pairs(source, arg, &hash.elements(), config);
        }
        if let Some(kw_hash) = arg.as_keyword_hash_node() {
            return self.check_pairs(source, arg, &kw_hash.elements(), config);
        }
        None
    }

    fn check_pairs(
        &self,
        source: &SourceFile,
        hash_arg: &ruby_prism::Node<'_>,
        pairs: &ruby_prism::NodeList<'_>,
        config: &CopConfig,
    ) -> Option<Diagnostic> {
        for element in pairs.iter() {
            let assoc = match element.as_assoc_node() {
                Some(a) => a,
                None => continue,
            };

            // Check if key is :type or `type:`
            let is_type_key = if let Some(sym) = assoc.key().as_symbol_node() {
                sym.unescaped() == b"type"
            } else {
                false
            };

            if !is_type_key {
                continue;
            }

            // Get the value as a symbol
            let type_sym = match assoc.value().as_symbol_node() {
                Some(s) => s,
                None => continue,
            };

            let type_name = type_sym.unescaped();
            let type_str = std::str::from_utf8(type_name.as_ref()).unwrap_or("");

            // Infer expected type from file path
            let file_path = source.path_str();
            let inferred = self.infer_type(file_path, config);

            if let Some(inferred_type) = inferred {
                if inferred_type == type_str {
                    let loc = if self.is_only_pair(pairs) {
                        hash_arg.location()
                    } else {
                        assoc.location()
                    };
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return Some(self.diagnostic(
                        source,
                        line,
                        column,
                        "Remove redundant spec type.".to_string(),
                    ));
                }
            }
        }
        None
    }

    fn infer_type(&self, file_path: &str, config: &CopConfig) -> Option<String> {
        // Check user-configured inferences first
        if let Some(inferences) = config.get_string_hash("Inferences") {
            for (prefix, inferred_type) in &inferences {
                let pattern = format!("spec/{prefix}/");
                if file_path.contains(&pattern) {
                    return Some(inferred_type.clone());
                }
            }
        }

        // Fall back to defaults
        for (prefix, inferred_type) in DEFAULT_INFERENCES {
            let pattern = format!("spec/{prefix}/");
            if file_path.contains(&pattern) {
                return Some(inferred_type.to_string());
            }
        }

        None
    }

    fn is_only_pair(&self, pairs: &ruby_prism::NodeList<'_>) -> bool {
        let count = pairs.iter().filter(|e| e.as_assoc_node().is_some()).count();
        count == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InferredSpecType, "cops/rspecrails/inferred_spec_type");
}
