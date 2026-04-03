use crate::cop::shared::node_type::{
    CALL_NODE, CLASS_NODE, MODULE_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::{HashMap, HashSet};

/// Checks for places where `attr_reader` and `attr_writer` for the same
/// method can be combined into a single `attr_accessor`.
///
/// ## Visibility scope tracking
///
/// RuboCop groups macros by their visibility scope (public/private/protected)
/// and only considers bisection within the same scope. For example:
///
/// ```ruby
/// class Foo
///   attr_reader :bar   # public scope
///   private
///   attr_writer :bar   # private scope -- NOT bisected
/// end
/// ```
///
/// This cop mirrors that behavior by tracking the current visibility as it
/// iterates through the class/module body statements. A bare `private`,
/// `protected`, or `public` call (with no arguments) changes the visibility
/// for all subsequent statements. Calls with arguments (e.g., `private :foo`)
/// do not change the ambient visibility.
///
/// ## Root cause of historical FPs (92 FP in corpus)
///
/// The original implementation did not track visibility scopes at all. It
/// collected all `attr_reader` and `attr_writer` calls in a class body
/// regardless of their position relative to `private`/`protected`/`public`
/// calls, then bisected them. This caused false positives whenever a reader
/// and writer for the same attribute were in different visibility scopes.
pub struct BisectedAttrAccessor;

/// Visibility scope for attr macros
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Visibility {
    Public,
    Private,
    Protected,
}

/// An attr_reader or attr_writer occurrence with its visibility and location
struct AttrOccurrence {
    name: String,
    visibility: Visibility,
    line: usize,
    column: usize,
}

impl Cop for BisectedAttrAccessor {
    fn name(&self) -> &'static str {
        "Style/BisectedAttrAccessor"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_NODE,
            MODULE_NODE,
            SINGLETON_CLASS_NODE,
            STATEMENTS_NODE,
            SYMBOL_NODE,
        ]
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
        let body = if let Some(class_node) = node.as_class_node() {
            class_node.body()
        } else if let Some(module_node) = node.as_module_node() {
            module_node.body()
        } else if let Some(sclass_node) = node.as_singleton_class_node() {
            sclass_node.body()
        } else {
            return;
        };

        let body = match body {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let mut readers: Vec<AttrOccurrence> = Vec::new();
        let mut writers: Vec<AttrOccurrence> = Vec::new();
        let mut current_visibility = Visibility::Public;

        for stmt in stmts.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
                if call.receiver().is_some() {
                    continue;
                }

                // Check for visibility-changing calls (bare private/protected/public with no args)
                if (name == "private" || name == "protected" || name == "public")
                    && call.arguments().is_none()
                    && call.block().is_none()
                {
                    current_visibility = match name {
                        "private" => Visibility::Private,
                        "protected" => Visibility::Protected,
                        "public" => Visibility::Public,
                        _ => unreachable!(),
                    };
                    continue;
                }

                let is_reader = name == "attr_reader" || name == "attr";
                let is_writer = name == "attr_writer";

                if !is_reader && !is_writer {
                    continue;
                }

                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        let attr_name = if let Some(sym) = arg.as_symbol_node() {
                            std::str::from_utf8(sym.unescaped())
                                .unwrap_or("")
                                .to_string()
                        } else {
                            continue;
                        };

                        let loc = arg.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());

                        let occurrence = AttrOccurrence {
                            name: attr_name,
                            visibility: current_visibility,
                            line,
                            column,
                        };

                        if is_reader {
                            readers.push(occurrence);
                        } else {
                            writers.push(occurrence);
                        }
                    }
                }
            }
        }

        // Group by visibility and find bisections within each scope
        let mut reader_names_by_vis: HashMap<Visibility, HashSet<String>> = HashMap::new();
        let mut writer_names_by_vis: HashMap<Visibility, HashSet<String>> = HashMap::new();

        for r in &readers {
            reader_names_by_vis
                .entry(r.visibility)
                .or_default()
                .insert(r.name.clone());
        }
        for w in &writers {
            writer_names_by_vis
                .entry(w.visibility)
                .or_default()
                .insert(w.name.clone());
        }

        // Find common names within each visibility scope
        let mut common: HashSet<(Visibility, String)> = HashSet::new();
        for (vis, reader_names) in &reader_names_by_vis {
            if let Some(writer_names) = writer_names_by_vis.get(vis) {
                for name in reader_names.intersection(writer_names) {
                    common.insert((*vis, name.clone()));
                }
            }
        }

        // Report diagnostics for bisected attrs
        for occ in readers.iter().chain(writers.iter()) {
            if common.contains(&(occ.visibility, occ.name.clone())) {
                diagnostics.push(self.diagnostic(
                    source,
                    occ.line,
                    occ.column,
                    format!("Combine both accessors into `attr_accessor :{}`.", occ.name),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BisectedAttrAccessor, "cops/style/bisected_attr_accessor");
}
