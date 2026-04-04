use ruby_prism::Visit;

use crate::cop::rake::RAKE_DEFAULT_INCLUDE;
use crate::cop::shared::method_dispatch_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that Rake tasks have a description defined with the `desc` method.
///
/// Tasks without descriptions don't appear in `rake -T` output and lack
/// documentation. The `:default` task is exempt.
///
/// RuboCop's corpus invocation runs with an external baseline config against
/// repo directories, which lints `*.rake` files but does not inspect
/// extensionless `Rakefile` paths. Nitrocop previously flagged those
/// `Rakefile` tasks and created corpus-only false positives, so this cop
/// skips files whose basename is exactly `Rakefile`.
pub struct Desc;

impl Cop for Desc {
    fn name(&self) -> &'static str {
        "Rake/Desc"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        RAKE_DEFAULT_INCLUDE
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if source
            .path
            .file_name()
            .is_some_and(|name| name == "Rakefile")
        {
            return;
        }

        let mut visitor = DescVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct DescVisitor<'a> {
    cop: &'a Desc,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl DescVisitor<'_> {
    /// Check if a task call is for the `:default` task.
    fn is_default_task(call: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(name) = crate::cop::rake::extract_task_name(call) {
            return name == "default";
        }
        false
    }

    /// Check if a task call has array prerequisites (e.g., `task default: [:test]`).
    /// Tasks defined only with prerequisites (hash-style) are exempt.
    fn has_only_prerequisites(call: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            // If the only argument is a keyword hash (e.g., `task default: [:test]`)
            if arg_list.len() == 1 {
                if let Some(hash) = arg_list[0].as_keyword_hash_node() {
                    for elem in hash.elements().iter() {
                        if let Some(assoc) = elem.as_assoc_node() {
                            // Check if the value is an array (prerequisites)
                            if assoc.value().as_array_node().is_some() {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if a given statement node is a `desc` call.
    fn is_desc_call(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            return method_dispatch_predicates::is_command(&call, b"desc");
        }
        false
    }

    /// Check siblings of a task call in a statements list.
    /// Returns true if a `desc` call precedes the task.
    fn has_preceding_desc(stmts: &[ruby_prism::Node<'_>], task_offset: usize) -> bool {
        if task_offset == 0 {
            return false;
        }
        // Check the immediately preceding sibling
        Self::is_desc_call(&stmts[task_offset - 1])
    }

    fn check_statements(&mut self, stmts: &ruby_prism::StatementsNode<'_>) {
        let body: Vec<_> = stmts.body().iter().collect();
        for (i, stmt) in body.iter().enumerate() {
            let call = if let Some(c) = stmt.as_call_node() {
                c
            } else {
                continue;
            };

            if call.name().as_slice() != b"task" || call.receiver().is_some() {
                continue;
            }

            // Skip :default task
            if Self::is_default_task(&call) {
                continue;
            }

            // Skip tasks with only prerequisites (hash-only args with array values)
            if Self::has_only_prerequisites(&call) {
                continue;
            }

            if !Self::has_preceding_desc(&body, i) {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Describe the task with `desc` method.".to_string(),
                ));
            }
        }
    }
}

impl<'pr> Visit<'pr> for DescVisitor<'_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        self.check_statements(node);
        ruby_prism::visit_statements_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Desc, "cops/rake/desc");

    #[test]
    fn ignores_root_rakefile_paths() {
        let source = b"task :foo\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &Desc,
            source,
            CopConfig::default(),
            "/tmp/repo/Rakefile",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn ignores_nested_rakefile_paths() {
        let source = b"task :foo\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &Desc,
            source,
            CopConfig::default(),
            "/tmp/repo/lib/glimmer/Rakefile",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn still_flags_rake_task_files() {
        let source = b"task :foo\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &Desc,
            source,
            CopConfig::default(),
            "/tmp/repo/lib/tasks/build.rake",
        );

        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    }
}
