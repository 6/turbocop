use crate::cop::util::class_body_calls;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CLASS_NODE, SYMBOL_NODE};

pub struct AfterCommitOverride;

const AFTER_COMMIT_METHODS: &[&[u8]] = &[
    b"after_commit",
    b"after_create_commit",
    b"after_update_commit",
    b"after_destroy_commit",
    b"after_save_commit",
];

impl Cop for AfterCommitOverride {
    fn name(&self) -> &'static str {
        "Rails/AfterCommitOverride"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let calls = class_body_calls(&class_node);
        // Collect after_commit calls that have a symbol as first argument
        let after_commit_calls: Vec<_> = calls
            .iter()
            .filter(|c| {
                c.receiver().is_none()
                    && AFTER_COMMIT_METHODS.contains(&c.name().as_slice())
            })
            .filter(|c| {
                // Only consider calls with a symbol first argument (named callbacks)
                if let Some(args) = c.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if let Some(first) = arg_list.first() {
                        return first.as_symbol_node().is_some();
                    }
                }
                false
            })
            .collect();

        // Group by callback name and flag duplicates
        let mut seen: std::collections::HashMap<Vec<u8>, bool> = std::collections::HashMap::new();

        for call in &after_commit_calls {
            let args = call.arguments().unwrap();
            let arg_list: Vec<_> = args.arguments().iter().collect();
            let sym = arg_list[0].as_symbol_node().unwrap();
            let name = sym.unescaped().to_vec();

            if seen.contains_key(&name) {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let name_str = String::from_utf8_lossy(&name);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "There can only be one `after_*_commit :{}` hook defined for a model.",
                        name_str
                    ),
                ));
            } else {
                seen.insert(name, true);
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AfterCommitOverride, "cops/rails/after_commit_override");
}
