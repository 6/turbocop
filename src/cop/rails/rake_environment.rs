use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};

pub struct RakeEnvironment;

impl Cop for RakeEnvironment {
    fn name(&self) -> &'static str {
        "Rails/RakeEnvironment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE]
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
        // Start from CallNode `task`, then check if it has a block
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be receiverless `task` call
        if call.receiver().is_some() {
            return;
        }

        if call.name().as_slice() != b"task" {
            return;
        }

        // Must have a block
        if call.block().is_none() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check if first arg is a symbol (simple task definition)
        let has_symbol_arg = arg_list[0].as_symbol_node().is_some();
        if !has_symbol_arg {
            return;
        }

        // Check if any argument is a hash with dependencies.
        // RuboCop checks for *any* dependency, not just :environment.
        // Dependency forms:
        //   task foo: :environment        (first arg is keyword hash)
        //   task :foo => :environment     (keyword hash with symbol value)
        //   task :foo, [:arg] => [:env]   (keyword hash with array value)
        for arg in &arg_list {
            if let Some(kw) = arg.as_keyword_hash_node() {
                for elem in kw.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node() {
                        let value = assoc.value();
                        // Symbol value: `=> :environment` or `foo: :environment`
                        if value.as_symbol_node().is_some() {
                            return;
                        }
                        // Array value: `=> [:environment]` or `=> [:env1, :env2]`
                        if let Some(arr) = value.as_array_node() {
                            if arr.elements().iter().next().is_some() {
                                return;
                            }
                        }
                    }
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Add `:environment` dependency to the rake task.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RakeEnvironment, "cops/rails/rake_environment");
}
