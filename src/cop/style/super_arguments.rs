use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct SuperArguments;

impl Cop for SuperArguments {
    fn name(&self) -> &'static str {
        "Style/SuperArguments"
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
        let mut visitor = SuperArgumentsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct SuperArgumentsVisitor<'a> {
    cop: &'a SuperArguments,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

/// Represents the kind of parameter in a method definition.
#[derive(Debug, PartialEq)]
enum DefParam {
    /// Required or optional positional param: `name` or `name = default`
    Positional(Vec<u8>),
    /// Rest param: `*args`
    Rest(Vec<u8>),
    /// Required or optional keyword param: `name:` or `name: default`
    Keyword(Vec<u8>),
    /// Keyword rest param: `**kwargs`
    KeywordRest(Vec<u8>),
    /// Block param: `&block`
    Block(Vec<u8>),
}

/// Extract the ordered list of def parameters with their kinds.
fn extract_def_params(params: &ruby_prism::ParametersNode<'_>) -> Vec<DefParam> {
    let mut result = Vec::new();

    for p in params.requireds().iter() {
        if let Some(rp) = p.as_required_parameter_node() {
            result.push(DefParam::Positional(rp.name().as_slice().to_vec()));
        }
    }
    for p in params.optionals().iter() {
        if let Some(op) = p.as_optional_parameter_node() {
            result.push(DefParam::Positional(op.name().as_slice().to_vec()));
        }
    }
    // Post params (after rest)
    for p in params.posts().iter() {
        if let Some(rp) = p.as_required_parameter_node() {
            result.push(DefParam::Positional(rp.name().as_slice().to_vec()));
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rp) = rest.as_rest_parameter_node() {
            if let Some(name) = rp.name() {
                result.push(DefParam::Rest(name.as_slice().to_vec()));
            }
        }
    }
    for p in params.keywords().iter() {
        if let Some(kw) = p.as_required_keyword_parameter_node() {
            let name = kw.name().as_slice();
            let clean = if name.ends_with(b":") { &name[..name.len()-1] } else { name };
            result.push(DefParam::Keyword(clean.to_vec()));
        }
        if let Some(kw) = p.as_optional_keyword_parameter_node() {
            let name = kw.name().as_slice();
            let clean = if name.ends_with(b":") { &name[..name.len()-1] } else { name };
            result.push(DefParam::Keyword(clean.to_vec()));
        }
    }
    if let Some(kw_rest) = params.keyword_rest() {
        if let Some(kwr) = kw_rest.as_keyword_rest_parameter_node() {
            if let Some(name) = kwr.name() {
                result.push(DefParam::KeywordRest(name.as_slice().to_vec()));
            }
        }
    }
    if let Some(block) = params.block() {
        if let Some(name) = block.name() {
            result.push(DefParam::Block(name.as_slice().to_vec()));
        }
    }
    result
}

/// Check if a super argument matches a def parameter.
fn super_arg_matches_def_param(arg: &ruby_prism::Node<'_>, def_param: &DefParam) -> bool {
    match def_param {
        DefParam::Positional(name) => {
            if let Some(lv) = arg.as_local_variable_read_node() {
                return lv.name().as_slice() == name.as_slice();
            }
            false
        }
        DefParam::Rest(name) => {
            if let Some(splat) = arg.as_splat_node() {
                if let Some(expr) = splat.expression() {
                    if let Some(lv) = expr.as_local_variable_read_node() {
                        return lv.name().as_slice() == name.as_slice();
                    }
                }
            }
            false
        }
        DefParam::Keyword(name) => {
            if let Some(assoc) = arg.as_assoc_node() {
                return keyword_pair_matches(&assoc, name);
            }
            false
        }
        DefParam::KeywordRest(name) => {
            if let Some(splat) = arg.as_assoc_splat_node() {
                if let Some(value) = splat.value() {
                    if let Some(lv) = value.as_local_variable_read_node() {
                        return lv.name().as_slice() == name.as_slice();
                    }
                }
            }
            false
        }
        DefParam::Block(name) => {
            if let Some(block_arg) = arg.as_block_argument_node() {
                if let Some(expr) = block_arg.expression() {
                    if let Some(lv) = expr.as_local_variable_read_node() {
                        return lv.name().as_slice() == name.as_slice();
                    }
                }
            }
            false
        }
    }
}

/// Check if an AssocNode is `name: name` (symbol key matching a local variable value).
fn keyword_pair_matches(assoc: &ruby_prism::AssocNode<'_>, name: &[u8]) -> bool {
    let key = assoc.key();
    let value = assoc.value();
    if let Some(sym) = key.as_symbol_node() {
        if let Some(lv) = value.as_local_variable_read_node() {
            let sym_name: &[u8] = sym.unescaped().as_ref();
            return sym_name == name && lv.name().as_slice() == name;
        }
    }
    false
}

/// Flatten super arguments: expand bare keyword hashes into individual pairs.
fn flatten_super_args<'a>(args: impl Iterator<Item = ruby_prism::Node<'a>>) -> Vec<ruby_prism::Node<'a>> {
    let mut result = Vec::new();
    for arg in args {
        if let Some(kh) = arg.as_keyword_hash_node() {
            for elem in kh.elements().iter() {
                result.push(elem);
            }
        } else {
            result.push(arg);
        }
    }
    result
}

struct SuperChecker<'a> {
    def_params: &'a [DefParam],
    offsets: Vec<usize>,
}

impl<'pr> Visit<'pr> for SuperChecker<'_> {
    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        let super_args_raw: Vec<ruby_prism::Node<'_>> = if let Some(arguments) = node.arguments() {
            arguments.arguments().iter().collect()
        } else {
            Vec::new()
        };

        let has_explicit_block = node.block().is_some();

        // Build effective def params (exclude block param if super has explicit block)
        let effective_def_params: Vec<&DefParam> = if has_explicit_block {
            self.def_params.iter().filter(|p| !matches!(p, DefParam::Block(_))).collect()
        } else {
            self.def_params.iter().collect()
        };

        let flat_args = flatten_super_args(super_args_raw.into_iter());

        if flat_args.is_empty() && effective_def_params.is_empty() {
            return; // Both empty — no offense
        }

        if flat_args.len() != effective_def_params.len() {
            return;
        }

        let all_match = flat_args.iter().zip(effective_def_params.iter())
            .all(|(arg, param)| super_arg_matches_def_param(arg, param));

        if all_match {
            self.offsets.push(node.location().start_offset());
        }
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Don't recurse into nested defs
    }
}

impl<'pr> Visit<'pr> for SuperArgumentsVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let def_params = if let Some(params) = node.parameters() {
            extract_def_params(&params)
        } else {
            Vec::new()
        };

        if let Some(body) = node.body() {
            let mut checker = SuperChecker {
                def_params: &def_params,
                offsets: Vec::new(),
            };
            checker.visit(&body);

            for offset in checker.offsets {
                let (line, column) = self.source.offset_to_line_col(offset);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Call `super` without arguments and parentheses when the signature is identical.".to_string(),
                ));
            }
        }

        // Don't recurse into nested defs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SuperArguments, "cops/style/super_arguments");
}
