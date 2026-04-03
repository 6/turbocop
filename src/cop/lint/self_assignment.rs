use crate::cop::shared::node_type::{
    CALL_NODE, CLASS_VARIABLE_AND_WRITE_NODE, CLASS_VARIABLE_OR_WRITE_NODE,
    CLASS_VARIABLE_READ_NODE, CLASS_VARIABLE_WRITE_NODE, CONSTANT_AND_WRITE_NODE,
    CONSTANT_OR_WRITE_NODE, CONSTANT_PATH_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_READ_NODE,
    CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_AND_WRITE_NODE, GLOBAL_VARIABLE_OR_WRITE_NODE,
    GLOBAL_VARIABLE_READ_NODE, GLOBAL_VARIABLE_WRITE_NODE, INSTANCE_VARIABLE_AND_WRITE_NODE,
    INSTANCE_VARIABLE_OR_WRITE_NODE, INSTANCE_VARIABLE_READ_NODE, INSTANCE_VARIABLE_WRITE_NODE,
    LOCAL_VARIABLE_AND_WRITE_NODE, LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_READ_NODE,
    LOCAL_VARIABLE_WRITE_NODE, MULTI_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/SelfAssignment detects useless self-assignment like `x = x`, `foo ||= foo`,
/// `foo.bar = foo.bar`, and `foo["k"] = foo["k"]`.
///
/// Handles: simple variable writes, compound or/and writes (||=, &&=) for all variable
/// types, multi-write (`a, b = a, b` and `a, b = [a, b]`), attribute setter self-assignment
/// (`foo.bar = foo.bar`), and index self-assignment (`foo[k] = foo[k]`).
pub struct SelfAssignment;

impl SelfAssignment {
    fn emit(
        &self,
        source: &SourceFile,
        loc: ruby_prism::Location<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Self-assignment detected.".to_string(),
        ));
    }
}

/// Check if a node is a "literal-like" value suitable for source-text comparison
/// in index (`[]`/`[]=`) arguments. Returns true for string, symbol, integer, float,
/// constant read, local/instance/class/global variable read nodes.
fn is_literal_or_variable(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_constant_read_node().is_some()
        || node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
}

impl Cop for SelfAssignment {
    fn name(&self) -> &'static str {
        "Lint/SelfAssignment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_VARIABLE_AND_WRITE_NODE,
            CLASS_VARIABLE_OR_WRITE_NODE,
            CLASS_VARIABLE_READ_NODE,
            CLASS_VARIABLE_WRITE_NODE,
            CONSTANT_AND_WRITE_NODE,
            CONSTANT_OR_WRITE_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_READ_NODE,
            CONSTANT_WRITE_NODE,
            GLOBAL_VARIABLE_AND_WRITE_NODE,
            GLOBAL_VARIABLE_OR_WRITE_NODE,
            GLOBAL_VARIABLE_READ_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            INSTANCE_VARIABLE_AND_WRITE_NODE,
            INSTANCE_VARIABLE_OR_WRITE_NODE,
            INSTANCE_VARIABLE_READ_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LOCAL_VARIABLE_AND_WRITE_NODE,
            LOCAL_VARIABLE_OR_WRITE_NODE,
            LOCAL_VARIABLE_READ_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            MULTI_WRITE_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _allow_rbs = config.get_bool("AllowRBSInlineAnnotation", false);

        // Local variable: x = x
        if let Some(write) = node.as_local_variable_write_node() {
            if let Some(read) = write.value().as_local_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Instance variable: @x = @x
        if let Some(write) = node.as_instance_variable_write_node() {
            if let Some(read) = write.value().as_instance_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Class variable: @@x = @@x
        if let Some(write) = node.as_class_variable_write_node() {
            if let Some(read) = write.value().as_class_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Global variable: $x = $x
        if let Some(write) = node.as_global_variable_write_node() {
            if let Some(read) = write.value().as_global_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Constant: FOO = FOO
        if let Some(write) = node.as_constant_write_node() {
            if let Some(read) = write.value().as_constant_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Constant path: Mod::FOO = Mod::FOO
        if let Some(write) = node.as_constant_path_write_node() {
            let target = write.target();
            let value = write.value();
            if let Some(val_path) = value.as_constant_path_node() {
                let target_src = target.location().as_slice();
                let val_src = val_path.location().as_slice();
                if target_src == val_src {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Local variable compound: foo ||= foo, foo &&= foo
        if let Some(write) = node.as_local_variable_or_write_node() {
            if let Some(read) = write.value().as_local_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }
        if let Some(write) = node.as_local_variable_and_write_node() {
            if let Some(read) = write.value().as_local_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Instance variable compound: @x ||= @x, @x &&= @x
        if let Some(write) = node.as_instance_variable_or_write_node() {
            if let Some(read) = write.value().as_instance_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }
        if let Some(write) = node.as_instance_variable_and_write_node() {
            if let Some(read) = write.value().as_instance_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Class variable compound: @@x ||= @@x, @@x &&= @@x
        if let Some(write) = node.as_class_variable_or_write_node() {
            if let Some(read) = write.value().as_class_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }
        if let Some(write) = node.as_class_variable_and_write_node() {
            if let Some(read) = write.value().as_class_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Global variable compound: $x ||= $x, $x &&= $x
        if let Some(write) = node.as_global_variable_or_write_node() {
            if let Some(read) = write.value().as_global_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }
        if let Some(write) = node.as_global_variable_and_write_node() {
            if let Some(read) = write.value().as_global_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Constant compound: FOO ||= FOO, FOO &&= FOO
        if let Some(write) = node.as_constant_or_write_node() {
            if let Some(read) = write.value().as_constant_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }
        if let Some(write) = node.as_constant_and_write_node() {
            if let Some(read) = write.value().as_constant_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    self.emit(source, write.location(), diagnostics);
                }
            }
            return;
        }

        // Multi-write: foo, bar = foo, bar OR foo, bar = [foo, bar]
        if let Some(mw) = node.as_multi_write_node() {
            let lefts: Vec<_> = mw.lefts().iter().collect();
            let value = mw.value();

            // rest or rights present means it's not a simple parallel assignment
            if mw.rest().is_some() || mw.rights().iter().count() > 0 {
                return;
            }

            // Value must be an ArrayNode (both `a, b = a, b` and `a, b = [a, b]`
            // parse as ArrayNode in Prism)
            if let Some(arr) = value.as_array_node() {
                let elements: Vec<_> = arr.elements().iter().collect();
                if elements.len() != lefts.len() {
                    return;
                }
                // Check each target matches its corresponding value by source text
                let all_match = lefts.iter().zip(elements.iter()).all(|(target, val)| {
                    if let Some(lt) = target.as_local_variable_target_node() {
                        if let Some(rv) = val.as_local_variable_read_node() {
                            return lt.name().as_slice() == rv.name().as_slice();
                        }
                    }
                    false
                });
                if all_match {
                    self.emit(source, mw.location(), diagnostics);
                }
            }
            return;
        }

        // CallNode: attribute self-assignment (foo.bar = foo.bar) and
        // index self-assignment (foo["k"] = foo["k"])
        if let Some(call) = node.as_call_node() {
            let method = call.name().as_slice();

            // Must have a receiver
            let recv = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            if method == b"[]=" {
                // Index self-assignment: foo[key] = foo[key]
                // In Prism, `foo[k] = v` is CallNode with name `[]=`, receiver `foo`,
                // arguments [k, v]. We need: v is a CallNode `[]` on same receiver with
                // same key arguments.
                let args = match call.arguments() {
                    Some(a) => a,
                    None => return,
                };
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.is_empty() {
                    return;
                }

                // Last argument is the value being assigned
                let value = &arg_list[arg_list.len() - 1];
                let key_args = &arg_list[..arg_list.len() - 1];

                // Value must be a CallNode with method `[]` on same receiver
                let val_call = match value.as_call_node() {
                    Some(c) => c,
                    None => return,
                };
                if val_call.name().as_slice() != b"[]" {
                    return;
                }
                let val_recv = match val_call.receiver() {
                    Some(r) => r,
                    None => return,
                };

                // Compare receivers by source text
                if recv.location().as_slice() != val_recv.location().as_slice() {
                    return;
                }

                // Compare key arguments
                let val_args = match val_call.arguments() {
                    Some(a) => a,
                    None => {
                        // No args on RHS [] — only matches if LHS also has no key args
                        if key_args.is_empty() {
                            self.emit(source, call.location(), diagnostics);
                        }
                        return;
                    }
                };
                let val_arg_list: Vec<_> = val_args.arguments().iter().collect();
                if key_args.len() != val_arg_list.len() {
                    return;
                }

                // Each key arg must be a literal/variable and match by source text
                let all_match = key_args.iter().zip(val_arg_list.iter()).all(|(lhs, rhs)| {
                    is_literal_or_variable(lhs)
                        && is_literal_or_variable(rhs)
                        && lhs.location().as_slice() == rhs.location().as_slice()
                });

                if all_match {
                    self.emit(source, call.location(), diagnostics);
                }
            } else if method.ends_with(b"=")
                && method != b"=="
                && method != b"!="
                && method != b"==="
            {
                // Attribute self-assignment: foo.bar = foo.bar
                // Method name is `bar=`, must have call_operator (`.` or `&.`)
                if call.call_operator_loc().is_none() {
                    return;
                }

                let args = match call.arguments() {
                    Some(a) => a,
                    None => return,
                };
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    return;
                }

                let value = &arg_list[0];
                let val_call = match value.as_call_node() {
                    Some(c) => c,
                    None => return,
                };

                // RHS must have no arguments and no block
                if val_call.arguments().is_some() || val_call.block().is_some() {
                    return;
                }

                // RHS must have a receiver
                let val_recv = match val_call.receiver() {
                    Some(r) => r,
                    None => return,
                };

                // Compare method names: LHS is `bar=`, RHS should be `bar`
                let setter_name = &method[..method.len() - 1]; // strip trailing `=`
                if val_call.name().as_slice() != setter_name {
                    return;
                }

                // Compare receivers by source text
                if recv.location().as_slice() != val_recv.location().as_slice() {
                    return;
                }

                // Compare call operators (both `.` or both `&.`)
                let lhs_op = call.call_operator_loc().map(|l| l.as_slice());
                let rhs_op = val_call.call_operator_loc().map(|l| l.as_slice());
                if lhs_op != rhs_op {
                    return;
                }

                self.emit(source, call.location(), diagnostics);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SelfAssignment, "cops/lint/self_assignment");
}
