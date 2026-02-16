use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct TrivialAccessors;

/// Default AllowedMethods from vendor config (to_ary, to_a, to_c, ... to_sym).
const DEFAULT_ALLOWED: &[&[u8]] = &[
    b"to_ary",
    b"to_a",
    b"to_c",
    b"to_enum",
    b"to_h",
    b"to_hash",
    b"to_i",
    b"to_int",
    b"to_io",
    b"to_open",
    b"to_path",
    b"to_proc",
    b"to_r",
    b"to_regexp",
    b"to_str",
    b"to_s",
    b"to_sym",
    b"initialize",
];

/// What kind of enclosing scope we're in.
#[derive(Clone, Copy, PartialEq)]
enum ScopeKind {
    /// Inside a `class` or `class << self`
    Class,
    /// Inside a `module` — trivial accessors are skipped here
    Module,
    /// Top level (not inside class/module)
    TopLevel,
}

impl Cop for TrivialAccessors {
    fn name(&self) -> &'static str {
        "Style/TrivialAccessors"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let exact_name_match = config.get_bool("ExactNameMatch", true);
        let allow_predicates = config.get_bool("AllowPredicates", true);
        let allow_dsl_writers = config.get_bool("AllowDSLWriters", true);
        let ignore_class_methods = config.get_bool("IgnoreClassMethods", false);
        let allowed_methods = config.get_string_array("AllowedMethods");

        let mut visitor = TrivialAccessorsVisitor {
            cop: self,
            source,
            exact_name_match,
            allow_predicates,
            allow_dsl_writers,
            ignore_class_methods,
            allowed_methods: &allowed_methods,
            scope_stack: vec![ScopeKind::TopLevel],
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct TrivialAccessorsVisitor<'a> {
    cop: &'a TrivialAccessors,
    source: &'a SourceFile,
    exact_name_match: bool,
    allow_predicates: bool,
    allow_dsl_writers: bool,
    ignore_class_methods: bool,
    allowed_methods: &'a Option<Vec<String>>,
    scope_stack: Vec<ScopeKind>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> TrivialAccessorsVisitor<'a> {
    fn current_scope(&self) -> ScopeKind {
        *self.scope_stack.last().unwrap_or(&ScopeKind::TopLevel)
    }

    fn check_def(&mut self, def_node: &ruby_prism::DefNode<'_>) {
        // Skip if we're at top level (not inside any class/module)
        if self.current_scope() == ScopeKind::TopLevel {
            return;
        }

        // Skip if we're inside a module (vendor's in_module_or_instance_eval? check)
        if self.current_scope() == ScopeKind::Module {
            return;
        }

        // Skip endless methods (no end keyword)
        if def_node.end_keyword_loc().is_none() {
            return;
        }

        // Skip class methods (def self.foo) when IgnoreClassMethods is true
        if self.ignore_class_methods && def_node.receiver().is_some() {
            return;
        }

        let method_name = def_node.name();
        let name_bytes = method_name.as_slice();

        // Check allowed methods (config or defaults), plus always skip `initialize`
        let is_allowed = if let Some(allowed) = self.allowed_methods {
            allowed.iter().any(|m| m.as_bytes() == name_bytes) || name_bytes == b"initialize"
        } else {
            DEFAULT_ALLOWED.iter().any(|&m| m == name_bytes)
        };

        if is_allowed {
            return;
        }

        // Get body statements
        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().into_iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        let single_stmt = &body_nodes[0];

        // Check for trivial reader: `def foo; @foo; end`
        if let Some(ivar_read) = single_stmt.as_instance_variable_read_node() {
            let ivar_name = ivar_read.name();
            let ivar_bytes = ivar_name.as_slice();
            let ivar_without_at = &ivar_bytes[1..];

            // Skip if method has parameters
            if let Some(params) = def_node.parameters() {
                if !params.requireds().is_empty()
                    || !params.optionals().is_empty()
                    || params.rest().is_some()
                {
                    return;
                }
            }

            // AllowPredicates: skip `def foo?; @foo; end`
            if self.allow_predicates && name_bytes.ends_with(b"?") {
                return;
            }

            if self.exact_name_match && name_bytes != ivar_without_at {
                return;
            }

            let def_loc = def_node.def_keyword_loc();
            let (line, column) = self.source.offset_to_line_col(def_loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Use `attr_reader` to define trivial reader methods.".to_string(),
            ));
            return;
        }

        // Check for trivial writer: `def foo=(val); @foo = val; end`
        if let Some(ivar_write) = single_stmt.as_instance_variable_write_node() {
            let ivar_name = ivar_write.name();
            let ivar_bytes = ivar_name.as_slice();
            let ivar_without_at = &ivar_bytes[1..];

            let is_setter = name_bytes.ends_with(b"=");

            // AllowDSLWriters: if true, skip non-setter writers
            if self.allow_dsl_writers && !is_setter {
                return;
            }

            if is_setter {
                let name_without_eq = &name_bytes[..name_bytes.len() - 1];
                if self.exact_name_match && name_without_eq != ivar_without_at {
                    return;
                }
            } else if self.exact_name_match && name_bytes != ivar_without_at {
                return;
            }

            // Check that the value being assigned is the parameter
            if let Some(params) = def_node.parameters() {
                let requireds: Vec<_> = params.requireds().into_iter().collect();
                if requireds.len() == 1 {
                    let value = ivar_write.value();
                    if value.as_local_variable_read_node().is_none() {
                        return;
                    }
                } else {
                    return;
                }
            } else {
                return;
            }

            let def_loc = def_node.def_keyword_loc();
            let (line, column) = self.source.offset_to_line_col(def_loc.start_offset());
            let msg = "Use `attr_writer` to define trivial writer methods.";
            self.diagnostics
                .push(self.cop.diagnostic(self.source, line, column, msg.to_string()));
        }
    }
}

impl<'pr> Visit<'pr> for TrivialAccessorsVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        self.scope_stack.push(ScopeKind::Class);
        // Visit children
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.scope_stack.push(ScopeKind::Class);
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.scope_stack.push(ScopeKind::Module);
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.check_def(node);
        // Don't recurse into nested defs — they have their own scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrivialAccessors, "cops/style/trivial_accessors");
}
