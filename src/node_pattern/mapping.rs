//! Parser gem → Prism node type mapping table.
//!
//! Maps NodePattern DSL type names (e.g. `send`, `block`, `if`) to their
//! corresponding Prism node types and child accessors.

use std::collections::HashMap;

pub struct NodeMapping {
    pub parser_type: &'static str,
    pub prism_type: &'static str,
    pub cast_method: &'static str,
    pub child_accessors: &'static [(&'static str, &'static str)],
}

impl Clone for NodeMapping {
    fn clone(&self) -> Self {
        Self {
            parser_type: self.parser_type,
            prism_type: self.prism_type,
            cast_method: self.cast_method,
            child_accessors: self.child_accessors,
        }
    }
}

pub fn build_mapping_table() -> HashMap<&'static str, &'static NodeMapping> {
    let mappings: &[NodeMapping] = &[
        NodeMapping {
            parser_type: "send",
            prism_type: "CallNode",
            cast_method: "as_call_node",
            child_accessors: &[
                ("receiver", "receiver()"),
                ("method_name", "name()"),
                ("args", "arguments()"),
            ],
        },
        NodeMapping {
            parser_type: "csend",
            prism_type: "CallNode",
            cast_method: "as_call_node",
            child_accessors: &[
                ("receiver", "receiver()"),
                ("method_name", "name()"),
                ("args", "arguments()"),
            ],
        },
        NodeMapping {
            parser_type: "block",
            prism_type: "BlockNode",
            cast_method: "as_block_node",
            child_accessors: &[
                ("call", "call()"),
                ("params", "parameters()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "def",
            prism_type: "DefNode",
            cast_method: "as_def_node",
            child_accessors: &[
                ("name", "name()"),
                ("params", "parameters()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "defs",
            prism_type: "DefNode",
            cast_method: "as_def_node",
            child_accessors: &[
                ("recv", "receiver()"),
                ("name", "name()"),
                ("params", "parameters()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "const",
            prism_type: "ConstantReadNode",
            cast_method: "as_constant_read_node",
            child_accessors: &[("name", "name()")],
        },
        NodeMapping {
            parser_type: "begin",
            prism_type: "BeginNode",
            cast_method: "as_begin_node",
            child_accessors: &[("body", "statements()")],
        },
        NodeMapping {
            parser_type: "pair",
            prism_type: "AssocNode",
            cast_method: "as_assoc_node",
            child_accessors: &[("key", "key()"), ("value", "value()")],
        },
        NodeMapping {
            parser_type: "hash",
            prism_type: "HashNode",
            cast_method: "as_hash_node",
            child_accessors: &[("pairs", "elements()")],
        },
        NodeMapping {
            parser_type: "lvar",
            prism_type: "LocalVariableReadNode",
            cast_method: "as_local_variable_read_node",
            child_accessors: &[("name", "name()")],
        },
        NodeMapping {
            parser_type: "ivar",
            prism_type: "InstanceVariableReadNode",
            cast_method: "as_instance_variable_read_node",
            child_accessors: &[("name", "name()")],
        },
        NodeMapping {
            parser_type: "cvar",
            prism_type: "ClassVariableReadNode",
            cast_method: "as_class_variable_read_node",
            child_accessors: &[("name", "name()")],
        },
        NodeMapping {
            parser_type: "gvar",
            prism_type: "GlobalVariableReadNode",
            cast_method: "as_global_variable_read_node",
            child_accessors: &[("name", "name()")],
        },
        NodeMapping {
            parser_type: "sym",
            prism_type: "SymbolNode",
            cast_method: "as_symbol_node",
            child_accessors: &[("value", "value()")],
        },
        NodeMapping {
            parser_type: "str",
            prism_type: "StringNode",
            cast_method: "as_string_node",
            child_accessors: &[("content", "content()")],
        },
        NodeMapping {
            parser_type: "int",
            prism_type: "IntegerNode",
            cast_method: "as_integer_node",
            child_accessors: &[("value", "value()")],
        },
        NodeMapping {
            parser_type: "float",
            prism_type: "FloatNode",
            cast_method: "as_float_node",
            child_accessors: &[("value", "value()")],
        },
        NodeMapping {
            parser_type: "true",
            prism_type: "TrueNode",
            cast_method: "as_true_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "false",
            prism_type: "FalseNode",
            cast_method: "as_false_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "nil",
            prism_type: "NilNode",
            cast_method: "as_nil_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "self",
            prism_type: "SelfNode",
            cast_method: "as_self_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "array",
            prism_type: "ArrayNode",
            cast_method: "as_array_node",
            child_accessors: &[("elements", "elements()")],
        },
        NodeMapping {
            parser_type: "if",
            prism_type: "IfNode",
            cast_method: "as_if_node",
            child_accessors: &[
                ("cond", "predicate()"),
                ("body", "statements()"),
                ("else", "subsequent()"),
            ],
        },
        NodeMapping {
            parser_type: "case",
            prism_type: "CaseNode",
            cast_method: "as_case_node",
            child_accessors: &[
                ("expr", "predicate()"),
                ("whens", "conditions()"),
                ("else", "else_clause()"),
            ],
        },
        NodeMapping {
            parser_type: "when",
            prism_type: "WhenNode",
            cast_method: "as_when_node",
            child_accessors: &[("conds", "conditions()"), ("body", "statements()")],
        },
        NodeMapping {
            parser_type: "while",
            prism_type: "WhileNode",
            cast_method: "as_while_node",
            child_accessors: &[("cond", "predicate()"), ("body", "statements()")],
        },
        NodeMapping {
            parser_type: "until",
            prism_type: "UntilNode",
            cast_method: "as_until_node",
            child_accessors: &[("cond", "predicate()"), ("body", "statements()")],
        },
        NodeMapping {
            parser_type: "for",
            prism_type: "ForNode",
            cast_method: "as_for_node",
            child_accessors: &[
                ("var", "index()"),
                ("iter", "collection()"),
                ("body", "statements()"),
            ],
        },
        NodeMapping {
            parser_type: "return",
            prism_type: "ReturnNode",
            cast_method: "as_return_node",
            child_accessors: &[("args", "arguments()")],
        },
        NodeMapping {
            parser_type: "yield",
            prism_type: "YieldNode",
            cast_method: "as_yield_node",
            child_accessors: &[("args", "arguments()")],
        },
        NodeMapping {
            parser_type: "and",
            prism_type: "AndNode",
            cast_method: "as_and_node",
            child_accessors: &[("left", "left()"), ("right", "right()")],
        },
        NodeMapping {
            parser_type: "or",
            prism_type: "OrNode",
            cast_method: "as_or_node",
            child_accessors: &[("left", "left()"), ("right", "right()")],
        },
        NodeMapping {
            parser_type: "regexp",
            prism_type: "RegularExpressionNode",
            cast_method: "as_regular_expression_node",
            child_accessors: &[("content", "content()")],
        },
        NodeMapping {
            parser_type: "class",
            prism_type: "ClassNode",
            cast_method: "as_class_node",
            child_accessors: &[
                ("name", "constant_path()"),
                ("super", "superclass()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "module",
            prism_type: "ModuleNode",
            cast_method: "as_module_node",
            child_accessors: &[("name", "constant_path()"), ("body", "body()")],
        },
        NodeMapping {
            parser_type: "lvasgn",
            prism_type: "LocalVariableWriteNode",
            cast_method: "as_local_variable_write_node",
            child_accessors: &[("name", "name()"), ("value", "value()")],
        },
        NodeMapping {
            parser_type: "ivasgn",
            prism_type: "InstanceVariableWriteNode",
            cast_method: "as_instance_variable_write_node",
            child_accessors: &[("name", "name()"), ("value", "value()")],
        },
        NodeMapping {
            parser_type: "casgn",
            prism_type: "ConstantWriteNode",
            cast_method: "as_constant_write_node",
            child_accessors: &[("name", "name()"), ("value", "value()")],
        },
        NodeMapping {
            parser_type: "splat",
            prism_type: "SplatNode",
            cast_method: "as_splat_node",
            child_accessors: &[("expr", "expression()")],
        },
        NodeMapping {
            parser_type: "super",
            prism_type: "SuperNode",
            cast_method: "as_super_node",
            child_accessors: &[("args", "arguments()")],
        },
        NodeMapping {
            parser_type: "zsuper",
            prism_type: "ForwardingSuperNode",
            cast_method: "as_forwarding_super_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "lambda",
            prism_type: "LambdaNode",
            cast_method: "as_lambda_node",
            child_accessors: &[("params", "parameters()"), ("body", "body()")],
        },
        NodeMapping {
            parser_type: "dstr",
            prism_type: "InterpolatedStringNode",
            cast_method: "as_interpolated_string_node",
            child_accessors: &[("parts", "parts()")],
        },
        NodeMapping {
            parser_type: "dsym",
            prism_type: "InterpolatedSymbolNode",
            cast_method: "as_interpolated_symbol_node",
            child_accessors: &[("parts", "parts()")],
        },
        NodeMapping {
            parser_type: "args",
            prism_type: "ParametersNode",
            cast_method: "as_parameters_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "any_block",
            prism_type: "BlockNode",
            cast_method: "as_block_node",
            child_accessors: &[
                ("call", "call()"),
                ("params", "parameters()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "cbase",
            prism_type: "ConstantPathNode",
            cast_method: "as_constant_path_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "op-asgn",
            prism_type: "OperatorWriteNode",
            cast_method: "as_operator_write_node",
            child_accessors: &[
                ("target", "target()"),
                ("operator", "binary_operator()"),
                ("value", "value()"),
            ],
        },
    ];

    // SAFETY: We leak these mappings for 'static lifetime. In the library context
    // this is a one-time allocation that lives for the process duration.
    let leaked: &'static [NodeMapping] = Box::leak(mappings.to_vec().into_boxed_slice());

    let mut table = HashMap::new();
    for mapping in leaked {
        table.insert(mapping.parser_type, mapping as &'static NodeMapping);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapping_table_completeness() {
        let table = build_mapping_table();
        for expected in &[
            "send", "csend", "block", "def", "defs", "const", "begin", "pair", "hash", "lvar",
            "ivar", "sym", "str", "int", "float", "true", "false", "nil", "self", "array", "if",
            "case", "when", "while", "until", "for", "return", "yield", "and", "or", "regexp",
            "class", "module", "lvasgn", "ivasgn", "casgn", "splat", "super", "zsuper", "lambda",
            "dstr", "dsym",
        ] {
            assert!(
                table.contains_key(expected),
                "Mapping table missing key node type: {expected}"
            );
        }
    }

    #[test]
    fn test_mapping_send_is_call_node() {
        let table = build_mapping_table();
        let send = table.get("send").unwrap();
        assert_eq!(send.prism_type, "CallNode");
        assert_eq!(send.cast_method, "as_call_node");
        assert!(!send.child_accessors.is_empty());
    }

    #[test]
    fn test_mapping_csend_same_as_send() {
        let table = build_mapping_table();
        let send = table.get("send").unwrap();
        let csend = table.get("csend").unwrap();
        assert_eq!(
            send.prism_type, csend.prism_type,
            "send and csend should map to same Prism type"
        );
        assert_eq!(send.cast_method, csend.cast_method);
    }
}
