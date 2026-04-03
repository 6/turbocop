use std::collections::HashMap;

use ruby_prism::Visit;

use crate::cop::shared::method_dispatch_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DuplicatedGroup;

impl Cop for DuplicatedGroup {
    fn name(&self) -> &'static str {
        "Bundler/DuplicatedGroup"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
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
        let mut visitor = GroupDeclarationVisitor {
            source,
            declarations: Vec::new(),
            source_scope_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());

        let mut seen: HashMap<String, usize> = HashMap::new();

        for declaration in visitor.declarations {
            if let Some(&first_line) = seen.get(&declaration.key) {
                diagnostics.push(self.diagnostic(
                    source,
                    declaration.line,
                    declaration.column,
                    format!(
                        "Gem group `{}` already defined on line {} of the Gemfile.",
                        declaration.group_name, first_line
                    ),
                ));
            } else {
                seen.insert(declaration.key, declaration.line);
            }
        }
    }
}

struct GroupDeclaration {
    key: String,
    group_name: String,
    line: usize,
    column: usize,
}

struct GroupDeclarationVisitor<'a> {
    source: &'a SourceFile,
    declarations: Vec<GroupDeclaration>,
    source_scope_stack: Vec<Option<String>>,
}

impl GroupDeclarationVisitor<'_> {
    fn nearest_source_scope_key(&self) -> Option<&str> {
        self.source_scope_stack
            .iter()
            .rev()
            .find_map(|scope| scope.as_deref())
    }
}

impl<'pr> Visit<'pr> for GroupDeclarationVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if method_dispatch_predicates::is_command(node, b"group") {
            let mut attributes = group_attributes(self.source, node);
            attributes.sort();

            let mut key = String::new();
            if let Some(scope_key) = self.nearest_source_scope_key() {
                key.push_str(scope_key);
            }
            key.push_str(&attributes.join(""));

            let group_name = group_display_name(self.source, node);
            let loc = node.message_loc().unwrap_or(node.location());
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());

            self.declarations.push(GroupDeclaration {
                key,
                group_name,
                line,
                column,
            });
        }

        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                self.source_scope_stack
                    .push(source_scope_key_for_call(self.source, node));
                ruby_prism::visit_block_node(self, &block_node);
                self.source_scope_stack.pop();

                if let Some(receiver) = node.receiver() {
                    self.visit(&receiver);
                }
                if let Some(arguments) = node.arguments() {
                    self.visit_arguments_node(&arguments);
                }
                return;
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

fn source_text(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    String::from_utf8_lossy(&source.as_bytes()[loc.start_offset()..loc.end_offset()]).into_owned()
}

fn source_scope_key_for_call(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
) -> Option<String> {
    if call.receiver().is_some() {
        return None;
    }

    let method_name = call.name().as_slice();
    if method_name != b"source"
        && method_name != b"git"
        && method_name != b"platforms"
        && method_name != b"path"
    {
        return None;
    }

    let method = std::str::from_utf8(method_name).ok()?;
    let first_arg = call
        .arguments()
        .and_then(|args| args.arguments().iter().next())
        .map(|arg| source_text(source, &arg))
        .unwrap_or_default();

    Some(format!("{method}{first_arg}"))
}

fn group_display_name(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> String {
    let Some(arguments) = call.arguments() else {
        return String::new();
    };

    let parts: Vec<String> = arguments
        .arguments()
        .iter()
        .map(|arg| source_text(source, &arg))
        .collect();
    parts.join(", ")
}

fn group_attributes(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Vec<String> {
    let Some(arguments) = call.arguments() else {
        return Vec::new();
    };

    let mut attrs = Vec::new();

    for argument in arguments.arguments().iter() {
        if let Some(kw_hash) = argument.as_keyword_hash_node() {
            let mut pairs: Vec<String> = kw_hash
                .elements()
                .iter()
                .filter_map(|elem| elem.as_assoc_node().map(|_| source_text(source, &elem)))
                .collect();
            pairs.sort();
            attrs.push(pairs.join(", "));
            continue;
        }

        if let Some(hash) = argument.as_hash_node() {
            let mut pairs: Vec<String> = hash
                .elements()
                .iter()
                .filter_map(|elem| elem.as_assoc_node().map(|_| source_text(source, &elem)))
                .collect();
            pairs.sort();
            attrs.push(pairs.join(", "));
            continue;
        }

        if let Some(symbol) = argument.as_symbol_node() {
            attrs.push(String::from_utf8_lossy(symbol.unescaped()).into_owned());
            continue;
        }

        if let Some(string) = argument.as_string_node() {
            attrs.push(String::from_utf8_lossy(string.unescaped()).into_owned());
            continue;
        }

        attrs.push(source_text(source, &argument));
    }

    attrs
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicatedGroup, "cops/bundler/duplicated_group");
}
