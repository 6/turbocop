use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrivialAccessors;

impl Cop for TrivialAccessors {
    fn name(&self) -> &'static str {
        "Style/TrivialAccessors"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let exact_name_match = config.get_bool("ExactNameMatch", true);
        let allow_predicates = config.get_bool("AllowPredicates", true);
        let allow_dsl_writers = config.get_bool("AllowDSLWriters", true);
        let _ignore_class_methods = config.get_bool("IgnoreClassMethods", false);
        let _allowed_methods = config.get_string_array("AllowedMethods");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Skip endless methods (no end keyword)
        if def_node.end_keyword_loc().is_none() {
            return Vec::new();
        }

        let method_name = def_node.name();
        let name_bytes = method_name.as_slice();
        let _name_str = String::from_utf8_lossy(name_bytes).to_string();

        // Get body statements
        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().into_iter().collect();
        if body_nodes.len() != 1 {
            return Vec::new();
        }

        let single_stmt = &body_nodes[0];

        // Check for trivial reader: `def foo; @foo; end`
        if let Some(ivar_read) = single_stmt.as_instance_variable_read_node() {
            let ivar_name = ivar_read.name();
            let ivar_bytes = ivar_name.as_slice();
            // ivar_bytes includes the @, e.g., "@foo"
            let ivar_without_at = &ivar_bytes[1..];

            // Skip if method has parameters
            if def_node.parameters().is_some() {
                if let Some(params) = def_node.parameters() {
                    if !params.requireds().is_empty()
                        || !params.optionals().is_empty()
                        || params.rest().is_some()
                    {
                        return Vec::new();
                    }
                }
            }

            // AllowPredicates: skip `def foo?; @foo; end`
            if allow_predicates && name_bytes.ends_with(b"?") {
                return Vec::new();
            }

            if exact_name_match && name_bytes != ivar_without_at {
                return Vec::new();
            }

            let def_loc = def_node.def_keyword_loc();
            let (line, column) = source.offset_to_line_col(def_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use `attr_reader` to define trivial reader methods.".to_string(),
            )];
        }

        // Check for trivial writer: `def foo=(val); @foo = val; end`
        if let Some(ivar_write) = single_stmt.as_instance_variable_write_node() {
            let ivar_name = ivar_write.name();
            let ivar_bytes = ivar_name.as_slice();
            let ivar_without_at = &ivar_bytes[1..];

            // Must be a setter method (name ends with `=`)
            let is_setter = name_bytes.ends_with(b"=");

            // AllowDSLWriters: if true, skip non-setter writers
            if allow_dsl_writers && !is_setter {
                return Vec::new();
            }

            if is_setter {
                let name_without_eq = &name_bytes[..name_bytes.len() - 1];
                if exact_name_match && name_without_eq != ivar_without_at {
                    return Vec::new();
                }
            } else if exact_name_match && name_bytes != ivar_without_at {
                return Vec::new();
            }

            // Check that the value being assigned is the parameter
            if let Some(params) = def_node.parameters() {
                let requireds: Vec<_> = params.requireds().into_iter().collect();
                if requireds.len() == 1 {
                    let value = ivar_write.value();
                    if let Some(local_read) = value.as_local_variable_read_node() {
                        let _param_name = local_read.name();
                        // Just check there's a simple assignment
                    } else {
                        return Vec::new();
                    }
                } else {
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }

            let def_loc = def_node.def_keyword_loc();
            let (line, column) = source.offset_to_line_col(def_loc.start_offset());
            let msg = if is_setter {
                "Use `attr_writer` to define trivial writer methods."
            } else {
                "Use `attr_writer` to define trivial writer methods."
            };
            return vec![self.diagnostic(source, line, column, msg.to_string())];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrivialAccessors, "cops/style/trivial_accessors");
}
