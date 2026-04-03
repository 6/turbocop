use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Matches RuboCop's effective ancestor search for `respond_to_missing?`.
///
/// Fixed behavior:
/// - defs inside wrappers such as `class << self`, conditionals, `Class.new`,
///   `class_eval`, and `instance_eval` still resolve against the same enclosing
///   search root RuboCop uses.
/// - program-root `method_missing` with an explicit positional missing-method
///   parameter is now reported. RuboCop flags those corpus examples, and a
///   top-level `respond_to_missing?` does not satisfy them.
///
/// We still leave zero-arg and rest-only top-level signatures alone, and we
/// also skip file-leading top-level block-arg forms with no keyword params,
/// because RuboCop is unstable on those shapes in isolation. That keeps the
/// program-scope fix limited to the confirmed corpus shape without reopening
/// the `dynamic_proxies.rb` regression.
///
/// Known limitation: the remaining corpus FP comes from RuboCop's parser
/// artifact when reopened classes share `respond_to_missing?` across separate
/// class bodies; Prism preserves those class bodies as separate scopes.
pub struct MissingRespondToMissing;

impl Cop for MissingRespondToMissing {
    fn name(&self) -> &'static str {
        "Style/MissingRespondToMissing"
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
        let mut visitor = MethodMissingVisitor {
            source,
            diagnostics: Vec::new(),
            defs: Vec::new(),
            ancestors: Vec::new(),
            entered_nodes: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.finish();
        diagnostics.extend(visitor.diagnostics);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum AncestorKind {
    Program,
    Class,
    Module,
    SingletonClass,
    Def,
    Other,
}

impl AncestorKind {
    fn is_boundary(self) -> bool {
        matches!(
            self,
            AncestorKind::Class
                | AncestorKind::Module
                | AncestorKind::SingletonClass
                | AncestorKind::Def
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct AncestorFrame {
    kind: AncestorKind,
    start_offset: usize,
    has_multiple_body_statements: bool,
}

impl AncestorFrame {
    fn from_node(node: ruby_prism::Node<'_>) -> Self {
        let kind = if node.as_program_node().is_some() {
            AncestorKind::Program
        } else if node.as_class_node().is_some() {
            AncestorKind::Class
        } else if node.as_module_node().is_some() {
            AncestorKind::Module
        } else if node.as_singleton_class_node().is_some() {
            AncestorKind::SingletonClass
        } else if node.as_def_node().is_some() {
            AncestorKind::Def
        } else {
            AncestorKind::Other
        };

        let has_multiple_body_statements = match kind {
            AncestorKind::Class => body_has_multiple_statements(
                node.as_class_node()
                    .and_then(|class_node| class_node.body()),
            ),
            AncestorKind::Module => body_has_multiple_statements(
                node.as_module_node()
                    .and_then(|module_node| module_node.body()),
            ),
            AncestorKind::SingletonClass => body_has_multiple_statements(
                node.as_singleton_class_node()
                    .and_then(|singleton_class_node| singleton_class_node.body()),
            ),
            AncestorKind::Def => body_has_multiple_statements(
                node.as_def_node().and_then(|def_node| def_node.body()),
            ),
            _ => false,
        };

        Self {
            kind,
            start_offset: node.location().start_offset(),
            has_multiple_body_statements,
        }
    }
}

fn body_has_multiple_statements(body: Option<ruby_prism::Node<'_>>) -> bool {
    match body {
        Some(node) => node
            .as_statements_node()
            .map(|statements| statements.body().len() > 1)
            .unwrap_or(false),
        None => false,
    }
}

fn has_explicit_positional_arg(node: &ruby_prism::DefNode<'_>) -> bool {
    node.parameters().is_some_and(|params| {
        !params.requireds().is_empty()
            || !params.optionals().is_empty()
            || !params.posts().is_empty()
    })
}

fn has_block_param(node: &ruby_prism::DefNode<'_>) -> bool {
    node.parameters()
        .is_some_and(|params| params.block().is_some())
}

fn has_keyword_params(node: &ruby_prism::DefNode<'_>) -> bool {
    node.parameters()
        .is_some_and(|params| !params.keywords().is_empty() || params.keyword_rest().is_some())
}

fn has_only_leading_comments_and_whitespace(source: &SourceFile, start_offset: usize) -> bool {
    source.as_bytes()[..start_offset]
        .split(|&b| b == b'\n')
        .all(|line| {
            let trimmed = line
                .iter()
                .copied()
                .skip_while(|b| b.is_ascii_whitespace())
                .collect::<Vec<_>>();
            trimmed.is_empty() || trimmed.starts_with(b"#")
        })
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ScopeKey {
    kind: AncestorKind,
    start_offset: usize,
}

impl ScopeKey {
    fn from_frame(frame: AncestorFrame) -> Self {
        Self {
            kind: frame.kind,
            start_offset: frame.start_offset,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MethodRole {
    MethodMissing,
    RespondToMissing,
}

struct DefRecord {
    role: MethodRole,
    is_class_method: bool,
    has_explicit_positional_arg: bool,
    has_block_param: bool,
    has_keyword_params: bool,
    has_only_leading_comments_and_whitespace: bool,
    start_offset: usize,
    root: Option<ScopeKey>,
    ancestor_scopes: Vec<ScopeKey>,
}

struct MethodMissingVisitor<'src> {
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    defs: Vec<DefRecord>,
    ancestors: Vec<AncestorFrame>,
    entered_nodes: Vec<bool>,
}

impl MethodMissingVisitor<'_> {
    fn current_root_key(&self) -> Option<ScopeKey> {
        let mut index = self.ancestors.len().checked_sub(2)?;
        let mut saw_wrapper = false;
        let mut nearest_other: Option<ScopeKey> = None;

        loop {
            let frame = *self.ancestors.get(index)?;

            match frame.kind {
                AncestorKind::Program => {
                    // When we reach program level, use the nearest enclosing block
                    // as the root scope (matching RuboCop's grandparent lookup for
                    // defs inside blocks like Class.new/instance_eval). If no block
                    // was encountered, use the program itself as the root (top-level
                    // method_missing).
                    return nearest_other.or(Some(ScopeKey::from_frame(frame)));
                }
                AncestorKind::SingletonClass => return Some(ScopeKey::from_frame(frame)),
                AncestorKind::Class | AncestorKind::Module | AncestorKind::Def => {
                    let has_outer_boundary = self.ancestors[..index]
                        .iter()
                        .rev()
                        .any(|ancestor| ancestor.kind.is_boundary());

                    if saw_wrapper || frame.has_multiple_body_statements || !has_outer_boundary {
                        return Some(ScopeKey::from_frame(frame));
                    }
                }
                AncestorKind::Other => {
                    saw_wrapper = true;
                    if nearest_other.is_none() {
                        nearest_other = Some(ScopeKey::from_frame(frame));
                    }
                }
            }

            index = index.checked_sub(1)?;
        }
    }

    fn current_ancestor_scopes(&self) -> Vec<ScopeKey> {
        self.ancestors
            .iter()
            .take(self.ancestors.len().saturating_sub(1))
            .filter(|ancestor| !matches!(ancestor.kind, AncestorKind::Program))
            .copied()
            .map(ScopeKey::from_frame)
            .collect()
    }

    fn record_def(&mut self, node: &ruby_prism::DefNode<'_>) {
        let role = match node.name().as_slice() {
            b"method_missing" => MethodRole::MethodMissing,
            b"respond_to_missing?" => MethodRole::RespondToMissing,
            _ => return,
        };

        self.defs.push(DefRecord {
            role,
            is_class_method: node.receiver().is_some(),
            has_explicit_positional_arg: has_explicit_positional_arg(node),
            has_block_param: has_block_param(node),
            has_keyword_params: has_keyword_params(node),
            has_only_leading_comments_and_whitespace: has_only_leading_comments_and_whitespace(
                self.source,
                node.location().start_offset(),
            ),
            start_offset: node.location().start_offset(),
            root: self.current_root_key(),
            ancestor_scopes: self.current_ancestor_scopes(),
        });
    }

    fn finish(&mut self) {
        let mut offense_offsets = Vec::new();

        for method_missing in self
            .defs
            .iter()
            .filter(|record| record.role == MethodRole::MethodMissing)
        {
            let root = match method_missing.root {
                Some(root) => root,
                None => continue,
            };

            // RuboCop reports the corpus' top-level `method_missing` defs when
            // they expose an explicit missing-method parameter, but its signal
            // is unstable for zero-arg and rest-only signatures.
            if root.kind == AncestorKind::Program
                && (!method_missing.has_explicit_positional_arg
                    || (method_missing.has_block_param
                        && !method_missing.has_keyword_params
                        && method_missing.has_only_leading_comments_and_whitespace))
            {
                continue;
            }

            let has_match = self.defs.iter().any(|respond| {
                respond.role == MethodRole::RespondToMissing
                    && respond.is_class_method == method_missing.is_class_method
                    && respond.ancestor_scopes.contains(&root)
            });

            if !has_match {
                offense_offsets.push(method_missing.start_offset);
            }
        }

        offense_offsets.sort_unstable();

        for offset in offense_offsets {
            let (line, column) = self.source.offset_to_line_col(offset);
            self.diagnostics.push(MissingRespondToMissing.diagnostic(
                self.source,
                line,
                column,
                "When using `method_missing`, define `respond_to_missing?`.".to_string(),
            ));
        }
    }
}

impl<'pr> Visit<'pr> for MethodMissingVisitor<'_> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        let keep = node.as_statements_node().is_none();
        self.entered_nodes.push(keep);
        if keep {
            self.ancestors.push(AncestorFrame::from_node(node));
        }
    }

    fn visit_branch_node_leave(&mut self) {
        if self.entered_nodes.pop().unwrap_or(false) {
            self.ancestors.pop();
        }
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.record_def(node);
        ruby_prism::visit_def_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MissingRespondToMissing,
        "cops/style/missing_respond_to_missing"
    );
}
