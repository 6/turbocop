use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks that access modifiers are declared in the correct style (group or inline).
///
/// ## Investigation (2026-03-31)
///
/// Root cause of the remaining FNs was twofold:
/// 1. nitrocop only batch-checked class/module/sclass bodies, so it missed inline access
///    modifiers in top-level code and in multi-statement block bodies such as
///    `each do |m| private m; public m end`.
/// 2. nitrocop only treated `private def foo` and symbol forms as "inline", but RuboCop's
///    group-style logic treats any non-allowed access-modifier send with arguments as
///    inline, including `private m` and `public target`.
///
/// Fix: process every `StatementsNode` in group style, simulate RuboCop's Parser-parent
/// behavior for single-child block/if bodies so one-line DSL blocks and one-line
/// conditionals stay ignored, and classify any non-allowed access modifier with
/// arguments as inline while preserving RuboCop's right-sibling suppression.
/// Also stop propagating macro scope through `BeginNode` wrappers that carry
/// rescue/ensure clauses; RuboCop treats those like `rescue` parents, so
/// `private def` after a class-level `begin ... rescue/ensure ... end` is allowed.
pub struct AccessModifierDeclarations;

const ACCESS_MODIFIERS: &[&str] = &["private", "protected", "public", "module_function"];

#[derive(Clone, Copy, Eq, PartialEq)]
enum StatementsOwnerKind {
    Other,
    Root,
    Block,
    If,
}

struct AccessModifierVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a AccessModifierDeclarations,
    enforced_style: &'a str,
    allow_modifiers_on_symbols: bool,
    allow_modifiers_on_attrs: bool,
    allow_modifiers_on_alias_method: bool,
    diagnostics: Vec<Diagnostic>,
    /// true when the current scope is a class/module/sclass body (not a nested block)
    in_class_body: bool,
    /// Synthetic owner kind for the next statements node we visit.
    statements_owner_kind: StatementsOwnerKind,
    /// Matches RuboCop's notion of being inside a macro scope.
    in_macro_scope: bool,
    /// Number of nested block wrappers under the current macro scope.
    macro_block_depth: usize,
}

struct ModifierClassification<'a> {
    method_name: &'a str,
    is_inlined: bool,
    is_symbol_pattern: bool,
}

/// Classify an access modifier call. Returns metadata for non-allowed access
/// modifier sends, or None when the call should be skipped entirely.
fn classify_access_modifier<'pr>(
    call: &ruby_prism::CallNode<'pr>,
    allow_modifiers_on_symbols: bool,
    allow_modifiers_on_attrs: bool,
    allow_modifiers_on_alias_method: bool,
) -> Option<ModifierClassification<'pr>> {
    let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
    if !ACCESS_MODIFIERS.contains(&method_name) || call.receiver().is_some() {
        return None;
    }

    let args = match call.arguments() {
        Some(arguments) => arguments,
        None => {
            return Some(ModifierClassification {
                method_name,
                is_inlined: false,
                is_symbol_pattern: false,
            });
        }
    };

    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return Some(ModifierClassification {
            method_name,
            is_inlined: false,
            is_symbol_pattern: false,
        });
    }

    let is_symbol_pattern = access_modifier_with_symbol(&arg_list);
    if is_symbol_pattern && allow_modifiers_on_symbols {
        return None;
    }

    let first_arg = &arg_list[0];
    if allow_modifiers_on_attrs {
        if let Some(inner_call) = first_arg.as_call_node() {
            let inner_name = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
            if matches!(
                inner_name,
                "attr_reader" | "attr_writer" | "attr_accessor" | "attr"
            ) {
                return None;
            }
        }
    }

    if allow_modifiers_on_alias_method {
        if let Some(inner_call) = first_arg.as_call_node() {
            let inner_name = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
            if inner_name == "alias_method" {
                return None;
            }
        }
    }

    Some(ModifierClassification {
        method_name,
        is_inlined: true,
        is_symbol_pattern,
    })
}

fn access_modifier_with_symbol(args: &[ruby_prism::Node<'_>]) -> bool {
    !args.is_empty()
        && (args.iter().all(|arg| arg.as_symbol_node().is_some())
            || (args.len() == 1 && symbol_splat_arg(&args[0])))
}

fn symbol_splat_arg(arg: &ruby_prism::Node<'_>) -> bool {
    let Some(splat) = arg.as_splat_node() else {
        return false;
    };

    let Some(expression) = splat.expression() else {
        return false;
    };

    expression
        .as_array_node()
        .is_some_and(|array| is_percent_symbol_array(&array))
        || expression.as_constant_read_node().is_some()
        || expression.as_constant_path_node().is_some()
        || expression.as_call_node().is_some()
}

fn is_percent_symbol_array(array: &ruby_prism::ArrayNode<'_>) -> bool {
    let Some(opening_loc) = array.opening_loc() else {
        return false;
    };

    let opening = opening_loc.as_slice();
    opening.starts_with(b"%i") || opening.starts_with(b"%I")
}

fn has_corresponding_def_nodes<'pr>(
    classification: &ModifierClassification<'pr>,
    args: &[ruby_prism::Node<'pr>],
    stmts: &[ruby_prism::Node<'pr>],
) -> bool {
    if !classification.is_symbol_pattern {
        return true;
    }

    let method_names: Vec<Vec<u8>> = args
        .iter()
        .filter_map(|arg| arg.as_symbol_node())
        .map(|sym| sym.unescaped().to_vec())
        .collect();

    if method_names.is_empty() {
        return false;
    }

    let defined_names: Vec<Vec<u8>> = stmts
        .iter()
        .filter_map(|stmt| stmt.as_def_node())
        .map(|def| def.name_loc().as_slice().to_vec())
        .collect();

    method_names
        .iter()
        .all(|method_name| defined_names.contains(method_name))
}

/// Info about an access modifier at a given position in a body's statement list.
struct ModifierInfo<'a> {
    method_name: &'a str,
    is_inlined: bool,
    has_corresponding_def_nodes: bool,
    start_offset: usize,
}

impl AccessModifierVisitor<'_> {
    fn check_group_style_statements<'pr>(&mut self, stmts: &[ruby_prism::Node<'pr>]) {
        if self.enforced_style != "group" || !self.in_macro_scope {
            return;
        }

        let direct_parent_is_block =
            matches!(self.statements_owner_kind, StatementsOwnerKind::Block) && stmts.len() == 1;
        let direct_parent_is_if =
            matches!(self.statements_owner_kind, StatementsOwnerKind::If) && stmts.len() == 1;
        let root_statements = matches!(self.statements_owner_kind, StatementsOwnerKind::Root);

        let infos: Vec<Option<ModifierInfo>> = stmts
            .iter()
            .map(|stmt| {
                let call = stmt.as_call_node()?;
                let classification = classify_access_modifier(
                    &call,
                    self.allow_modifiers_on_symbols,
                    self.allow_modifiers_on_attrs,
                    self.allow_modifiers_on_alias_method,
                )?;

                if direct_parent_is_block || direct_parent_is_if {
                    return None;
                }

                if root_statements && classification.is_symbol_pattern {
                    return None;
                }

                let args = call.arguments()?;
                let arg_list: Vec<_> = args.arguments().iter().collect();

                Some(ModifierInfo {
                    method_name: classification.method_name,
                    is_inlined: classification.is_inlined,
                    has_corresponding_def_nodes: has_corresponding_def_nodes(
                        &classification,
                        &arg_list,
                        stmts,
                    ),
                    start_offset: call.location().start_offset(),
                })
            })
            .collect();

        for (index, info) in infos.iter().enumerate() {
            let Some(info) = info else {
                continue;
            };

            if !info.is_inlined {
                continue;
            }

            let has_right_sibling_same_inline_modifier = infos[index + 1..].iter().any(|other| {
                matches!(
                    other,
                    Some(other_info)
                        if other_info.is_inlined
                            && other_info.has_corresponding_def_nodes
                            && other_info.method_name == info.method_name
                )
            });

            if has_right_sibling_same_inline_modifier {
                continue;
            }

            let (line, column) = self.source.offset_to_line_col(info.start_offset);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!(
                    "`{}` should not be inlined in method definitions.",
                    info.method_name
                ),
            ));
        }
    }
}

impl<'pr> Visit<'pr> for AccessModifierVisitor<'_> {
    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        let saved = self.statements_owner_kind;
        let saved_macro_scope = self.in_macro_scope;
        let saved_macro_block_depth = self.macro_block_depth;
        self.statements_owner_kind = StatementsOwnerKind::Root;
        self.in_macro_scope = true;
        self.macro_block_depth = 0;
        ruby_prism::visit_program_node(self, node);
        self.statements_owner_kind = saved;
        self.in_macro_scope = saved_macro_scope;
        self.macro_block_depth = saved_macro_block_depth;
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let stmts: Vec<_> = node.body().iter().collect();
        self.check_group_style_statements(&stmts);

        let saved = self.statements_owner_kind;
        self.statements_owner_kind = StatementsOwnerKind::Other;
        ruby_prism::visit_statements_node(self, node);
        self.statements_owner_kind = saved;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let saved_in_class_body = self.in_class_body;
        let saved_owner = self.statements_owner_kind;
        let saved_macro_scope = self.in_macro_scope;
        let saved_macro_block_depth = self.macro_block_depth;
        self.in_class_body = true;
        self.statements_owner_kind = StatementsOwnerKind::Other;
        self.in_macro_scope = true;
        self.macro_block_depth = 0;
        ruby_prism::visit_class_node(self, node);
        self.statements_owner_kind = saved_owner;
        self.in_class_body = saved_in_class_body;
        self.in_macro_scope = saved_macro_scope;
        self.macro_block_depth = saved_macro_block_depth;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let saved_in_class_body = self.in_class_body;
        let saved_owner = self.statements_owner_kind;
        let saved_macro_scope = self.in_macro_scope;
        let saved_macro_block_depth = self.macro_block_depth;
        self.in_class_body = true;
        self.statements_owner_kind = StatementsOwnerKind::Other;
        self.in_macro_scope = true;
        self.macro_block_depth = 0;
        ruby_prism::visit_module_node(self, node);
        self.statements_owner_kind = saved_owner;
        self.in_class_body = saved_in_class_body;
        self.in_macro_scope = saved_macro_scope;
        self.macro_block_depth = saved_macro_block_depth;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let saved_in_class_body = self.in_class_body;
        let saved_owner = self.statements_owner_kind;
        let saved_macro_scope = self.in_macro_scope;
        let saved_macro_block_depth = self.macro_block_depth;
        self.in_class_body = true;
        self.statements_owner_kind = StatementsOwnerKind::Other;
        self.in_macro_scope = true;
        self.macro_block_depth = 0;
        ruby_prism::visit_singleton_class_node(self, node);
        self.statements_owner_kind = saved_owner;
        self.in_class_body = saved_in_class_body;
        self.in_macro_scope = saved_macro_scope;
        self.macro_block_depth = saved_macro_block_depth;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let saved_in_class_body = self.in_class_body;
        let saved_owner = self.statements_owner_kind;
        let saved_macro_scope = self.in_macro_scope;
        let saved_macro_block_depth = self.macro_block_depth;
        self.in_class_body = false;
        self.statements_owner_kind = StatementsOwnerKind::Block;
        self.in_macro_scope = saved_macro_scope && saved_macro_block_depth == 0;
        self.macro_block_depth = saved_macro_block_depth + 1;
        ruby_prism::visit_block_node(self, node);
        self.statements_owner_kind = saved_owner;
        self.in_class_body = saved_in_class_body;
        self.in_macro_scope = saved_macro_scope;
        self.macro_block_depth = saved_macro_block_depth;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let saved_in_class_body = self.in_class_body;
        let saved_owner = self.statements_owner_kind;
        let saved_macro_scope = self.in_macro_scope;
        let saved_macro_block_depth = self.macro_block_depth;
        self.in_class_body = false;
        self.statements_owner_kind = StatementsOwnerKind::Block;
        self.in_macro_scope = false;
        self.macro_block_depth = 0;
        ruby_prism::visit_lambda_node(self, node);
        self.statements_owner_kind = saved_owner;
        self.in_class_body = saved_in_class_body;
        self.in_macro_scope = saved_macro_scope;
        self.macro_block_depth = saved_macro_block_depth;
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let saved = self.statements_owner_kind;
        self.statements_owner_kind = StatementsOwnerKind::If;
        ruby_prism::visit_if_node(self, node);
        self.statements_owner_kind = saved;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let saved_macro_scope = self.in_macro_scope;
        let saved_macro_block_depth = self.macro_block_depth;

        let is_pure_begin = node.rescue_clause().is_none()
            && node.ensure_clause().is_none()
            && node.else_clause().is_none();
        if !is_pure_begin {
            self.in_macro_scope = false;
            self.macro_block_depth = 0;
        }

        ruby_prism::visit_begin_node(self, node);
        self.in_macro_scope = saved_macro_scope;
        self.macro_block_depth = saved_macro_block_depth;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let saved_macro_scope = self.in_macro_scope;
        let saved_macro_block_depth = self.macro_block_depth;
        self.in_macro_scope = false;
        self.macro_block_depth = 0;
        ruby_prism::visit_def_node(self, node);
        self.in_macro_scope = saved_macro_scope;
        self.macro_block_depth = saved_macro_block_depth;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // In group mode, direct modifiers are handled in visit_statements_node.
        // Here we keep the existing inline-style handling.
        if self.enforced_style == "inline" && self.in_class_body {
            let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
            if ACCESS_MODIFIERS.contains(&method_name)
                && node.receiver().is_none()
                && node.arguments().is_none()
            {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!("`{}` should not be used in a group style.", method_name),
                ));
            }
        }
        ruby_prism::visit_call_node(self, node);
    }
}

impl Cop for AccessModifierDeclarations {
    fn name(&self) -> &'static str {
        "Style/AccessModifierDeclarations"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "group");
        let allow_modifiers_on_symbols = config.get_bool("AllowModifiersOnSymbols", true);
        let allow_modifiers_on_attrs = config.get_bool("AllowModifiersOnAttrs", true);
        let allow_modifiers_on_alias_method = config.get_bool("AllowModifiersOnAliasMethod", true);

        let mut visitor = AccessModifierVisitor {
            source,
            cop: self,
            enforced_style,
            allow_modifiers_on_symbols,
            allow_modifiers_on_attrs,
            allow_modifiers_on_alias_method,
            diagnostics: Vec::new(),
            in_class_body: true,
            statements_owner_kind: StatementsOwnerKind::Other,
            in_macro_scope: false,
            macro_block_depth: 0,
        };

        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        AccessModifierDeclarations,
        "cops/style/access_modifier_declarations"
    );
}
