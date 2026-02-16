use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct StructNewOverride;

const STRUCT_METHOD_NAMES: &[&str] = &[
    "all?", "any?", "chain", "chunk", "chunk_while", "class", "clone", "collect",
    "collect_concat", "compact", "count", "cycle", "deconstruct", "deconstruct_keys",
    "detect", "dig", "display", "drop", "drop_while", "dup", "each", "each_cons",
    "each_entry", "each_pair", "each_slice", "each_with_index", "each_with_object",
    "entries", "enum_for", "eql?", "equal?", "extend", "filter", "filter_map", "find",
    "find_all", "find_index", "first", "flat_map", "freeze", "frozen?", "grep", "grep_v",
    "group_by", "hash", "include?", "inject", "inspect", "instance_eval", "instance_exec",
    "instance_of?", "instance_variable_defined?", "instance_variable_get",
    "instance_variable_set", "instance_variables", "is_a?", "itself", "kind_of?", "lazy",
    "length", "map", "max", "max_by", "member?", "members", "method", "methods", "min",
    "min_by", "minmax", "minmax_by", "nil?", "none?", "object_id", "one?", "partition",
    "private_methods", "protected_methods", "public_method", "public_methods", "public_send",
    "reduce", "reject", "remove_instance_variable", "respond_to?", "reverse_each", "select",
    "send", "singleton_class", "singleton_method", "singleton_methods", "size",
    "slice_after", "slice_before", "slice_when", "sort", "sort_by", "sum", "take",
    "take_while", "tally", "tap", "then", "to_a", "to_enum", "to_h", "to_s", "to_set",
    "uniq", "values", "values_at", "yield_self", "zip",
];

impl Cop for StructNewOverride {
    fn name(&self) -> &'static str {
        "Lint/StructNewOverride"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"new" {
            return Vec::new();
        }

        // Receiver must be Struct
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let is_struct = if let Some(const_read) = recv.as_constant_read_node() {
            const_read.name().as_slice() == b"Struct"
        } else if let Some(const_path) = recv.as_constant_path_node() {
            const_path
                .name()
                .is_some_and(|n| n.as_slice() == b"Struct")
                && const_path.parent().is_none()
        } else {
            false
        };

        if !is_struct {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();
        let arg_list: Vec<_> = args.arguments().iter().collect();

        for (index, arg) in arg_list.iter().enumerate() {
            // Skip first argument if it's a string (class name)
            if index == 0 {
                if arg.as_string_node().is_some() {
                    continue;
                }
            }

            // Get the member name from symbol or string
            let member_name = if let Some(sym) = arg.as_symbol_node() {
                std::str::from_utf8(&sym.unescaped()).ok().map(|s| s.to_string())
            } else if let Some(s) = arg.as_string_node() {
                std::str::from_utf8(&s.unescaped()).ok().map(|s| s.to_string())
            } else {
                None
            };

            if let Some(name) = member_name {
                if STRUCT_METHOD_NAMES.contains(&name.as_str()) {
                    let loc = arg.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "`:{}` member overrides `Struct#{}` and it may be unexpected.",
                            name, name
                        ),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StructNewOverride, "cops/lint/struct_new_override");
}
