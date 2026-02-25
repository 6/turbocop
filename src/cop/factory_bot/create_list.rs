use ruby_prism::Visit;

use crate::cop::factory_bot::{FACTORY_BOT_SPEC_INCLUDE, is_factory_call};
use crate::cop::node_type::{
    ARRAY_NODE, ASSOC_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, CONSTANT_PATH_NODE,
    CONSTANT_READ_NODE, HASH_NODE, INTEGER_NODE, KEYWORD_HASH_NODE, REQUIRED_PARAMETER_NODE,
    STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct CreateList;

impl Cop for CreateList {
    fn name(&self) -> &'static str {
        "FactoryBot/CreateList"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_SPEC_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            ASSOC_NODE,
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            HASH_NODE,
            INTEGER_NODE,
            KEYWORD_HASH_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
            STRING_NODE,
            SYMBOL_NODE,
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
        let style = config.get_str("EnforcedStyle", "create_list");
        let explicit_only = config.get_bool("ExplicitOnly", false);

        // Check array literals with repeated create calls
        if let Some(array) = node.as_array_node() {
            diagnostics.extend(self.check_array_literal(source, &array, style, explicit_only));
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if style == "create_list" {
            diagnostics.extend(self.check_for_create_list_style(source, &call, explicit_only));
        }

        if style == "n_times" {
            diagnostics.extend(self.check_for_n_times_style(source, &call, explicit_only));
        }
    }
}

impl CreateList {
    /// Check array literals like `[create(:user), create(:user)]` for repeated identical create calls.
    fn check_array_literal(
        &self,
        source: &SourceFile,
        array: &ruby_prism::ArrayNode<'_>,
        style: &str,
        explicit_only: bool,
    ) -> Vec<Diagnostic> {
        let elements: Vec<_> = array.elements().iter().collect();
        if elements.len() < 2 {
            return Vec::new();
        }

        // All elements must be create calls
        let mut create_calls = Vec::new();
        for elem in &elements {
            let call = match elem.as_call_node() {
                Some(c) => c,
                None => return Vec::new(),
            };
            if call.name().as_slice() != b"create" {
                return Vec::new();
            }
            if !is_factory_call(call.receiver(), explicit_only) {
                return Vec::new();
            }
            create_calls.push(call);
        }

        // All create calls must have the same source representation of arguments
        let first = &create_calls[0];
        let first_args_src = get_args_source(first, source);

        for call in &create_calls[1..] {
            if get_args_source(call, source) != first_args_src {
                return Vec::new();
            }
        }

        // If the first argument is an interpolated symbol (e.g. :"canonical_#{type}"),
        // the calls may produce different values at runtime despite identical source text.
        if let Some(args) = first.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if !arg_list.is_empty() && arg_list[0].as_interpolated_symbol_node().is_some() {
                return Vec::new();
            }
        }

        // Check if args contain method calls — in that case, suggest n.times.map instead
        let has_method_calls = if let Some(args) = first.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            arguments_include_method_call(&arg_list)
        } else {
            false
        };

        let loc = array.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let count = elements.len();

        let msg = if has_method_calls || style == "n_times" {
            format!("Prefer {}.times.map.", count)
        } else {
            "Prefer create_list.".to_string()
        };

        vec![self.diagnostic(source, line, column, msg)]
    }

    /// With create_list style: flag `n.times { create :user }` and `n.times.map { create ... }` blocks
    fn check_for_create_list_style(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        explicit_only: bool,
    ) -> Vec<Diagnostic> {
        // Check if this call has a block containing a single create
        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Extract repeat count from the call (n.times, n.times.map)
        let count = match get_repeat_count_from_source(call, source) {
            Some(c) if c > 1 => c,
            _ => return Vec::new(),
        };

        // Check if block arg is used
        if block_param_is_used(&block_node) {
            return Vec::new();
        }

        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Body must be a single factory create call (or create with a sub-block)
        let body_call = match get_single_create_call(&body) {
            Some(c) => c,
            None => return Vec::new(),
        };

        if body_call.name().as_slice() != b"create" {
            return Vec::new();
        }

        if !is_factory_call(body_call.receiver(), explicit_only) {
            return Vec::new();
        }

        // Must have arguments, first must be symbol
        let args = match body_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() || arg_list[0].as_symbol_node().is_none() {
            return Vec::new();
        }

        // Check if arguments include a method call (rand, etc.)
        if arguments_include_method_call(&arg_list) {
            return Vec::new();
        }

        // Check if arguments include value omission (Ruby 3.1+ `key:` shorthand)
        if arguments_include_value_omission(&arg_list) {
            return Vec::new();
        }

        let _ = count;
        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Prefer create_list.".to_string())]
    }

    /// With n_times style: flag `create_list :user, 3` calls
    fn check_for_n_times_style(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        explicit_only: bool,
    ) -> Vec<Diagnostic> {
        if call.name().as_slice() != b"create_list" {
            return Vec::new();
        }

        if !is_factory_call(call.receiver(), explicit_only) {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return Vec::new();
        }

        // First arg: symbol or string (factory name)
        if arg_list[0].as_symbol_node().is_none() && arg_list[0].as_string_node().is_none() {
            return Vec::new();
        }

        // Second arg: integer (count)
        let count = match get_integer_value(&arg_list[1], source) {
            Some(c) => c,
            None => return Vec::new(),
        };

        if count < 2 {
            return Vec::new();
        }

        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(source, line, column, format!("Prefer {}.times.map.", count))]
    }
}

/// Get repeat count from source bytes (since integer value parsing needs source).
fn get_repeat_count_from_source(
    call: &ruby_prism::CallNode<'_>,
    source: &SourceFile,
) -> Option<i64> {
    let method = call.name().as_slice();

    if method == b"times" {
        if let Some(recv) = call.receiver() {
            if let Some(int) = recv.as_integer_node() {
                let src =
                    &source.as_bytes()[int.location().start_offset()..int.location().end_offset()];
                if let Ok(s) = std::str::from_utf8(src) {
                    return s.parse::<i64>().ok();
                }
            }
        }
    }

    if method == b"map" {
        if let Some(recv) = call.receiver() {
            if let Some(times_call) = recv.as_call_node() {
                if times_call.name().as_slice() == b"times" {
                    if let Some(int_recv) = times_call.receiver() {
                        if let Some(int) = int_recv.as_integer_node() {
                            let src = &source.as_bytes()
                                [int.location().start_offset()..int.location().end_offset()];
                            if let Ok(s) = std::str::from_utf8(src) {
                                return s.parse::<i64>().ok();
                            }
                        }
                    }
                }
            }
        }
    }

    // Array.new(N) { ... } — receiver is `Array`, method is `new`, first arg is integer
    if method == b"new" {
        if let Some(recv) = call.receiver() {
            let is_array = recv
                .as_constant_read_node()
                .is_some_and(|c| c.name().as_slice() == b"Array")
                || recv.as_constant_path_node().is_some_and(|cp| {
                    let src = &source.as_bytes()
                        [cp.location().start_offset()..cp.location().end_offset()];
                    src == b"Array" || src == b"::Array"
                });
            if is_array {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 {
                        if let Some(int) = arg_list[0].as_integer_node() {
                            let src = &source.as_bytes()
                                [int.location().start_offset()..int.location().end_offset()];
                            if let Ok(s) = std::str::from_utf8(src) {
                                return s.parse::<i64>().ok();
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Check if a block has parameters that are used in the body.
fn block_param_is_used(block_node: &ruby_prism::BlockNode<'_>) -> bool {
    let params = match block_node.parameters() {
        Some(p) => p,
        None => return false,
    };

    let block_params = match params.as_block_parameters_node() {
        Some(bp) => bp,
        None => return false,
    };

    let inner_params = match block_params.parameters() {
        Some(p) => p,
        None => return false,
    };

    let param_names: Vec<Vec<u8>> = inner_params
        .requireds()
        .iter()
        .filter_map(|p| p.as_required_parameter_node())
        .map(|p| p.name().as_slice().to_vec())
        .collect();

    if param_names.is_empty() {
        return false;
    }

    let body = match block_node.body() {
        Some(b) => b,
        None => return false,
    };

    has_local_var_read(&body, &param_names)
}

fn has_local_var_read(node: &ruby_prism::Node<'_>, names: &[Vec<u8>]) -> bool {
    struct VarFinder {
        names: Vec<Vec<u8>>,
        found: bool,
    }

    impl<'pr> Visit<'pr> for VarFinder {
        fn visit_local_variable_read_node(
            &mut self,
            node: &ruby_prism::LocalVariableReadNode<'pr>,
        ) {
            if self
                .names
                .iter()
                .any(|n| node.name().as_slice() == n.as_slice())
            {
                self.found = true;
            }
        }
    }

    let mut finder = VarFinder {
        names: names.to_vec(),
        found: false,
    };
    finder.visit(node);
    finder.found
}

/// Get the single create call from a block body.
fn get_single_create_call<'a>(body: &ruby_prism::Node<'a>) -> Option<ruby_prism::CallNode<'a>> {
    if let Some(stmts) = body.as_statements_node() {
        let children: Vec<_> = stmts.body().iter().collect();
        if children.len() != 1 {
            return None;
        }
        return children[0].as_call_node();
    }

    // Single expression body
    body.as_call_node()
}

/// Check if arguments to create include a method call (like `rand`).
fn arguments_include_method_call(args: &[ruby_prism::Node<'_>]) -> bool {
    for arg in args.iter().skip(1) {
        if contains_send_node(arg) {
            return true;
        }
    }
    false
}

fn contains_send_node(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        // A method call with receiver, arguments, or block
        if call.receiver().is_some() || call.arguments().is_some() || call.block().is_some() {
            return true;
        }
        // A bare method call like `rand`
        return true;
    }

    // Check keyword hash values
    if let Some(hash) = node.as_keyword_hash_node() {
        for elem in hash.elements().iter() {
            if let Some(pair) = elem.as_assoc_node() {
                if contains_send_node(&pair.value()) {
                    return true;
                }
            }
        }
    }

    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            if let Some(pair) = elem.as_assoc_node() {
                if contains_send_node(&pair.value()) {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if arguments contain value omission (Ruby 3.1+ `key:` shorthand).
fn arguments_include_value_omission(args: &[ruby_prism::Node<'_>]) -> bool {
    for arg in args.iter().skip(1) {
        if let Some(hash) = arg.as_keyword_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(pair) = elem.as_assoc_node() {
                    if pair.value().as_implicit_node().is_some() {
                        return true;
                    }
                }
            }
        }
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(pair) = elem.as_assoc_node() {
                    if pair.value().as_implicit_node().is_some() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Get the source bytes of a call's arguments (for comparing create calls in arrays).
fn get_args_source<'a>(call: &ruby_prism::CallNode<'a>, source: &SourceFile) -> Vec<u8> {
    match call.arguments() {
        Some(args) => {
            let loc = args.location();
            source.as_bytes()[loc.start_offset()..loc.end_offset()].to_vec()
        }
        None => Vec::new(),
    }
}

/// Extract integer value from a node via source bytes.
fn get_integer_value(node: &ruby_prism::Node<'_>, source: &SourceFile) -> Option<i64> {
    if let Some(int) = node.as_integer_node() {
        let src = &source.as_bytes()[int.location().start_offset()..int.location().end_offset()];
        let s = std::str::from_utf8(src).ok()?;
        return s.parse::<i64>().ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CreateList, "cops/factorybot/create_list");
}
