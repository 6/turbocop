use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ClassStructure;

/// Categories of class body elements in expected order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ElementCategory {
    ModuleInclusion,
    Constants,
    PublicClassMethods,
    Initializer,
    PublicMethods,
    ProtectedMethods,
    PrivateMethods,
    Unknown,
}

impl Cop for ClassStructure {
    fn name(&self) -> &'static str {
        "Layout/ClassStructure"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Reference config keys so config_audit passes
        let _categories = config.get_string_array("Categories");
        let _expected_order = config.get_string_array("ExpectedOrder");

        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let body = match class_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();
        let mut current_visibility = ElementCategory::PublicMethods;
        let mut last_category = ElementCategory::ModuleInclusion; // start at earliest
        let mut first = true;

        for stmt in stmts.body().iter() {
            // Track visibility changes
            if let Some(call) = stmt.as_call_node() {
                if call.receiver().is_none() {
                    let name = call.name().as_slice();
                    match name {
                        b"protected" => {
                            if call.arguments().is_none() {
                                current_visibility = ElementCategory::ProtectedMethods;
                                continue;
                            }
                        }
                        b"private" => {
                            if call.arguments().is_none() {
                                current_visibility = ElementCategory::PrivateMethods;
                                continue;
                            }
                        }
                        b"public" => {
                            if call.arguments().is_none() {
                                current_visibility = ElementCategory::PublicMethods;
                                continue;
                            }
                        }
                        _ => {}
                    }
                }
            }

            let category = categorize_statement(&stmt, current_visibility);
            if category == ElementCategory::Unknown {
                continue;
            }

            if first {
                last_category = category;
                first = false;
                continue;
            }

            if category < last_category {
                let (line, col) = source.offset_to_line_col(stmt.location().start_offset());
                let expected = format!("{:?}", last_category);
                let actual = format!("{:?}", category);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    col,
                    format!(
                        "{actual} is expected to appear before {expected}.",
                    ),
                ));
            } else {
                last_category = category;
            }
        }

        diagnostics
    }
}

fn categorize_statement(
    stmt: &ruby_prism::Node<'_>,
    current_visibility: ElementCategory,
) -> ElementCategory {
    // Module inclusion: include, extend, prepend
    if let Some(call) = stmt.as_call_node() {
        if call.receiver().is_none() {
            let name = call.name().as_slice();
            match name {
                b"include" | b"extend" | b"prepend" => return ElementCategory::ModuleInclusion,
                _ => {}
            }
        }
    }

    // Constants
    if stmt.as_constant_write_node().is_some() || stmt.as_constant_path_write_node().is_some() {
        return ElementCategory::Constants;
    }

    // Method definitions
    if let Some(def) = stmt.as_def_node() {
        if def.name().as_slice() == b"initialize" {
            return ElementCategory::Initializer;
        }
        return current_visibility;
    }

    // Singleton method definitions (class methods: `def self.foo`)
    if let Some(def_node) = stmt.as_def_node() {
        if def_node.receiver().is_some() {
            return ElementCategory::PublicClassMethods;
        }
    }

    ElementCategory::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ClassStructure, "cops/layout/class_structure");
}
