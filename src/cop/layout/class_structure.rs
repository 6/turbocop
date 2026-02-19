use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CLASS_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE, DEF_NODE, STATEMENTS_NODE, SYMBOL_NODE};

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

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CLASS_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE, DEF_NODE, STATEMENTS_NODE, SYMBOL_NODE]
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

        let all_stmts: Vec<_> = stmts.body().iter().collect();

        for (idx, stmt) in all_stmts.iter().enumerate() {
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

            let category = categorize_statement(stmt, current_visibility);
            if category == ElementCategory::Unknown {
                continue;
            }

            // Skip private constants (constants followed by private_constant :NAME)
            if category == ElementCategory::Constants {
                if is_private_constant(stmt, &all_stmts, idx) {
                    continue;
                }
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

    // Constants (only simple literal assignments, not dynamic)
    if stmt.as_constant_write_node().is_some() || stmt.as_constant_path_write_node().is_some() {
        return ElementCategory::Constants;
    }

    // Method definitions: check receiver first for class methods
    if let Some(def) = stmt.as_def_node() {
        if def.receiver().is_some() {
            return ElementCategory::PublicClassMethods;
        }
        if def.name().as_slice() == b"initialize" {
            return ElementCategory::Initializer;
        }
        return current_visibility;
    }

    ElementCategory::Unknown
}

/// Check if a constant assignment has a `private_constant :NAME` call among its siblings.
fn is_private_constant(
    stmt: &ruby_prism::Node<'_>,
    all_stmts: &[ruby_prism::Node<'_>],
    idx: usize,
) -> bool {
    // Get the constant name
    let const_name = if let Some(cw) = stmt.as_constant_write_node() {
        cw.name().as_slice().to_vec()
    } else if let Some(cpw) = stmt.as_constant_path_write_node() {
        // For constant path writes, get the last component
        let target = cpw.target();
        let bytes = target.location().as_slice();
        // Extract just the last name after ::
        if let Some(pos) = bytes.windows(2).rposition(|w| w == b"::") {
            bytes[pos + 2..].to_vec()
        } else {
            bytes.to_vec()
        }
    } else {
        return false;
    };

    // Check subsequent siblings for `private_constant :NAME`
    for sibling in &all_stmts[idx + 1..] {
        if let Some(call) = sibling.as_call_node() {
            if call.receiver().is_none() && call.name().as_slice() == b"private_constant" {
                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        if let Some(sym) = arg.as_symbol_node() {
                            if sym.unescaped() == const_name.as_slice() {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ClassStructure, "cops/layout/class_structure");
}
