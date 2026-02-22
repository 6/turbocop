#![allow(clippy::redundant_closure)]
//! NodePattern codegen — experimental prototype.
//!
//! Status: NOT used in CI or the standard cop-writing workflow. Kept in-tree
//! as a reference implementation of the NodePattern DSL parser.
//!
//! What works:
//!   - Lexer and parser for the NodePattern DSL
//!   - Parser→Prism node type mapping table
//!   - Code generation for simple single-type patterns without captures
//!
//! What does NOT work:
//!   - Alternatives codegen (e.g. `{send | csend}`)
//!   - Capture variables (`$name`)
//!   - Literal value matching (`:symbol`, `"string"`, integers)
//!   - `nil?` / `cbase` handling
//!   - Verify mode (stub — always reports "not implemented")
//!
//! For writing cops, the mapping table in `docs/node_pattern_analysis.md` is
//! the more useful reference — it shows which Prism node types and accessors
//! correspond to each Parser gem node type.
//!
//! Usage:
//!   cargo run --bin node_pattern_codegen -- generate <ruby_file>
//!   cargo run --bin node_pattern_codegen -- verify <ruby_file> <rust_file>

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::{self, Write};
use std::process;

use turbocop::node_pattern::{
    ExtractedPattern, Lexer, NodeMapping, Parser, PatternKind, PatternNode, build_mapping_table,
    extract_patterns, pattern_summary, walk_vendor_patterns,
};

// ---------------------------------------------------------------------------
// Rust Code Generator
// ---------------------------------------------------------------------------

struct CodeGenerator {
    mapping: HashMap<&'static str, &'static NodeMapping>,
    output: String,
    indent: usize,
    capture_count: usize,
    has_captures: bool,
    helper_stubs: Vec<String>,
}

impl CodeGenerator {
    fn new() -> Self {
        Self {
            mapping: build_mapping_table(),
            output: String::new(),
            indent: 0,
            capture_count: 0,
            has_captures: false,
            helper_stubs: Vec::new(),
        }
    }

    fn indent_str(&self) -> String {
        "    ".repeat(self.indent)
    }

    fn writeln(&mut self, s: &str) {
        let indent = self.indent_str();
        let _ = writeln!(self.output, "{indent}{s}");
    }

    /// Scan the pattern tree for captures to determine function signature.
    fn count_captures(node: &PatternNode) -> usize {
        match node {
            PatternNode::Capture(inner) => 1 + Self::count_captures(inner),
            PatternNode::NodeMatch { children, .. } => {
                children.iter().map(|c| Self::count_captures(c)).sum()
            }
            PatternNode::Alternatives(alts) => {
                // All branches must have same capture count; use max
                alts.iter()
                    .map(|a| Self::count_captures(a))
                    .max()
                    .unwrap_or(0)
            }
            PatternNode::Conjunction(items) => items.iter().map(|c| Self::count_captures(c)).sum(),
            PatternNode::Negation(inner) => Self::count_captures(inner),
            PatternNode::ParentRef(inner) => Self::count_captures(inner),
            PatternNode::DescendRef(inner) => Self::count_captures(inner),
            _ => 0,
        }
    }

    fn generate_pattern(&mut self, extracted: &ExtractedPattern, pattern: &PatternNode) -> String {
        self.output.clear();
        self.capture_count = 0;
        self.helper_stubs.clear();

        let num_captures = Self::count_captures(pattern);
        self.has_captures = num_captures > 0;

        let fn_name = extracted.method_name.trim_end_matches('?').to_string();

        // Generate function signature
        if self.has_captures {
            self.writeln(&format!(
                "fn {fn_name}<'a>(node: &ruby_prism::Node<'a>) -> Option<MatchCapture<'a>> {{"
            ));
        } else {
            self.writeln(&format!(
                "fn {fn_name}(node: &ruby_prism::Node<'_>) -> bool {{"
            ));
        }
        self.indent += 1;

        // Generate the body
        self.generate_node_check(pattern, "node", true);

        // Return success
        if self.has_captures {
            // The captures are returned within the generation
        } else {
            self.writeln("true");
        }

        self.indent -= 1;
        self.writeln("}");

        let mut full_output = String::new();

        // Add capture type if needed
        if self.has_captures {
            let _ = writeln!(full_output, "// Capture result from {fn_name}");
            if num_captures == 1 {
                let _ = writeln!(full_output, "type MatchCapture<'a> = ruby_prism::Node<'a>;");
            } else {
                let fields: Vec<String> = (0..num_captures)
                    .map(|_| "ruby_prism::Node<'a>".to_string())
                    .collect();
                let _ = writeln!(
                    full_output,
                    "type MatchCapture<'a> = ({});",
                    fields.join(", ")
                );
            }
            let _ = writeln!(full_output);
        }

        full_output.push_str(&self.output);

        // Add helper stubs
        for stub in &self.helper_stubs {
            let _ = writeln!(full_output);
            let _ = writeln!(full_output, "// Generated stub — cop must implement this");
            let _ = writeln!(
                full_output,
                "fn {stub}(node: &ruby_prism::Node<'_>) -> bool {{"
            );
            let _ = writeln!(full_output, "    todo!(\"implement #{stub} helper\")");
            let _ = writeln!(full_output, "}}");
        }

        full_output
    }

    fn generate_node_check(&mut self, node: &PatternNode, var: &str, is_top: bool) {
        match node {
            PatternNode::NodeMatch {
                node_type,
                children,
            } => {
                self.generate_node_match(node_type, children, var, is_top);
            }
            PatternNode::Wildcard => {
                // No check needed — any value is OK
            }
            PatternNode::Rest => {
                // No check on remaining children
            }
            PatternNode::NilPredicate => {
                // nil? means receiver is None
                if self.has_captures {
                    self.writeln(&format!("if {var}.is_some() {{ return None; }}"));
                } else {
                    self.writeln(&format!("if {var}.is_some() {{ return false; }}"));
                }
            }
            PatternNode::NilLiteral => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "if {var}.as_nil_node().is_none() {{ return {fail}; }}"
                ));
            }
            PatternNode::TrueLiteral => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "if {var}.as_true_node().is_none() {{ return {fail}; }}"
                ));
            }
            PatternNode::FalseLiteral => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "if {var}.as_false_node().is_none() {{ return {fail}; }}"
                ));
            }
            PatternNode::SymbolLiteral(name) => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!("// Check symbol :{name}"));
                self.writeln(&format!("if {var} != b\"{name}\" {{ return {fail}; }}"));
            }
            PatternNode::IntLiteral(n) => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!("if {var} != {n} {{ return {fail}; }}"));
            }
            PatternNode::StringLiteral(s) => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!("if {var} != b\"{s}\" {{ return {fail}; }}"));
            }
            PatternNode::HelperCall(name) => {
                let fail = if self.has_captures { "None" } else { "false" };
                let fn_name = name.trim_end_matches('?');
                self.writeln(&format!("if !{fn_name}(&{var}) {{ return {fail}; }}"));
                if !self.helper_stubs.contains(&fn_name.to_string()) {
                    self.helper_stubs.push(fn_name.to_string());
                }
            }
            PatternNode::Capture(inner) => {
                let cap_idx = self.capture_count;
                self.capture_count += 1;
                let cap_var = format!("capture_{cap_idx}");

                // First generate inner check, then capture the variable
                self.writeln(&format!("let {cap_var} = {var}.clone();"));
                self.generate_node_check(inner, var, false);

                // If this is the last statement before return, we use the capture
                if is_top {
                    self.writeln(&format!("return Some({cap_var});"));
                }
            }
            PatternNode::Alternatives(alts) => {
                self.generate_alternatives(alts, var);
            }
            PatternNode::Conjunction(items) => {
                for item in items {
                    self.generate_node_check(item, var, false);
                }
            }
            PatternNode::Negation(inner) => {
                self.generate_negation(inner, var);
            }
            PatternNode::TypePredicate(typ) => {
                let fail = if self.has_captures { "None" } else { "false" };
                let cast = self
                    .mapping
                    .get(typ.as_str())
                    .map(|m| m.cast_method.to_string());
                if let Some(cast) = cast {
                    self.writeln(&format!("if {var}.{cast}().is_none() {{ return {fail}; }}"));
                } else {
                    self.writeln(&format!("// Unknown type predicate: {typ}?"));
                }
            }
            PatternNode::ParamRef(param) => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!("// TODO: parameter reference %{param}"));
                self.writeln(&format!(
                    "// if {var} != param_{param} {{ return {fail}; }}"
                ));
            }
            PatternNode::ParentRef(inner) => {
                self.writeln("// TODO: parent node reference (^)");
                self.writeln(&format!("// Check parent: {:?}", pattern_summary(inner)));
            }
            PatternNode::DescendRef(inner) => {
                self.writeln("// TODO: descend operator (`)");
                self.writeln(&format!(
                    "// Descend into children looking for: {:?}",
                    pattern_summary(inner)
                ));
            }
            PatternNode::Ident(name) => {
                // An identifier in child position — might be a node type check
                let fail = if self.has_captures { "None" } else { "false" };
                let cast = self
                    .mapping
                    .get(name.as_str())
                    .map(|m| m.cast_method.to_string());
                if let Some(cast) = cast {
                    self.writeln(&format!("if {var}.{cast}().is_none() {{ return {fail}; }}"));
                } else {
                    self.writeln(&format!("// Unknown identifier: {name}"));
                }
            }
            PatternNode::FloatLiteral(s) => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!("// Float check: {s}"));
                self.writeln(&format!("// if {var}.value() != {s} {{ return {fail}; }}"));
            }
        }
    }

    fn generate_node_match(
        &mut self,
        node_type: &str,
        children: &[PatternNode],
        var: &str,
        _is_top: bool,
    ) {
        let fail = if self.has_captures {
            "return None"
        } else {
            "return false"
        };
        if node_type == "_complex" {
            self.writeln("// Complex pattern — manual review needed");
            for (i, child) in children.iter().enumerate() {
                self.writeln(&format!("// child[{i}]: {:?}", pattern_summary(child)));
            }
            return;
        }

        let Some(mapping) = self.mapping.get(node_type) else {
            self.writeln(&format!("// Unmapped node type: {node_type}"));
            self.writeln(&format!(
                "// TODO: add mapping for {node_type} to generate correct code"
            ));
            return;
        };

        // Copy mapping data to avoid borrow conflict with &mut self
        let cast = mapping.cast_method.to_string();
        let accessors: Vec<(&str, &str)> = mapping.child_accessors.to_vec();

        let typed_var = format!("{var}_{node_type}");

        // Cast the node
        if self.has_captures {
            self.writeln(&format!("let {typed_var} = {var}.{cast}()?;"));
        } else {
            self.writeln(&format!(
                "let Some({typed_var}) = {var}.{cast}() else {{ {fail}; }};"
            ));
        }

        // Special handling for csend — check call_operator
        if node_type == "csend" {
            self.writeln("// csend: verify safe navigation operator");
            self.writeln(&format!(
                "if {typed_var}.call_operator_loc().is_none() {{ {fail}; }}"
            ));
        }

        let mut accessor_idx = 0;

        for child in children {
            match child {
                PatternNode::Rest => {
                    // ... means skip remaining children
                    break;
                }
                PatternNode::Wildcard => {
                    // Skip this accessor — any value OK
                    accessor_idx += 1;
                }
                _ => {
                    if accessor_idx < accessors.len() {
                        let (_child_name, accessor) = accessors[accessor_idx];
                        let child_var = format!("{typed_var}_{accessor_idx}");

                        // Generate accessor call and child check
                        self.generate_child_access_and_check(
                            &typed_var,
                            accessor,
                            &child_var,
                            child,
                            node_type,
                            accessor_idx,
                        );
                        accessor_idx += 1;
                    } else {
                        self.writeln(&format!(
                            "// Extra child beyond known accessors: {:?}",
                            pattern_summary(child)
                        ));
                    }
                }
            }
        }
    }

    fn generate_child_access_and_check(
        &mut self,
        parent_var: &str,
        accessor: &str,
        child_var: &str,
        child: &PatternNode,
        parent_type: &str,
        child_idx: usize,
    ) {
        let fail = if self.has_captures { "None" } else { "false" };

        match child {
            PatternNode::NilPredicate => {
                // nil? on a child means that child should be None
                self.writeln(&format!(
                    "if {parent_var}.{accessor}.is_some() {{ return {fail}; }}"
                ));
            }
            PatternNode::SymbolLiteral(name) => {
                // Check method name or symbol value
                if accessor.contains("name") {
                    self.writeln(&format!(
                        "if {parent_var}.{accessor} != b\"{name}\" {{ return {fail}; }}"
                    ));
                } else {
                    // Might be a symbol node child — check value
                    self.writeln(&format!(
                        "// Check for symbol :{name} in {parent_type} child {child_idx}"
                    ));
                    self.writeln(&format!("let {child_var} = {parent_var}.{accessor};"));
                    self.writeln(&format!(
                        "// TODO: extract symbol value and compare to \"{name}\""
                    ));
                }
            }
            PatternNode::NodeMatch {
                node_type,
                children,
            } => {
                // Recurse into child node
                let is_optional = accessor.contains("receiver")
                    || accessor.contains("body")
                    || accessor.contains("subsequent")
                    || accessor.contains("superclass")
                    || accessor.contains("else_clause")
                    || accessor.contains("parameters");

                if is_optional {
                    if self.has_captures {
                        self.writeln(&format!(
                            "let {child_var} = {parent_var}.{accessor}.ok_or(())?;"
                        ));
                    } else {
                        self.writeln(&format!(
                            "let Some({child_var}) = {parent_var}.{accessor} else {{ return {fail}; }};"
                        ));
                    }
                } else {
                    self.writeln(&format!("let {child_var} = {parent_var}.{accessor};"));
                }
                self.generate_node_match(node_type, children, child_var, false);
            }
            PatternNode::Wildcard => {
                // No check needed
            }
            PatternNode::Rest => {
                // No check needed
            }
            PatternNode::Alternatives(alts) => {
                // For alternatives on a symbol/method name, generate OR checks
                if accessor.contains("name") {
                    self.generate_name_alternatives(parent_var, accessor, alts);
                } else {
                    self.writeln(&format!("let {child_var} = {parent_var}.{accessor};"));
                    self.generate_alternatives(alts, child_var);
                }
            }
            PatternNode::Capture(inner) => {
                let cap_idx = self.capture_count;
                self.capture_count += 1;
                self.writeln(&format!("let capture_{cap_idx} = {parent_var}.{accessor};"));
                // Still validate the inner pattern
                let temp_var = format!("{child_var}_cap");
                self.writeln(&format!("let {temp_var} = {parent_var}.{accessor};"));
                self.generate_node_check(inner, &temp_var, false);
            }
            PatternNode::HelperCall(name) => {
                let fn_name = name.trim_end_matches('?');
                self.writeln(&format!("let {child_var} = {parent_var}.{accessor};"));
                self.writeln(&format!("if !{fn_name}(&{child_var}) {{ return {fail}; }}"));
                if !self.helper_stubs.contains(&fn_name.to_string()) {
                    self.helper_stubs.push(fn_name.to_string());
                }
            }
            PatternNode::Negation(inner) => {
                self.writeln(&format!("let {child_var} = {parent_var}.{accessor};"));
                self.generate_negation(inner, child_var);
            }
            PatternNode::Conjunction(items) => {
                self.writeln(&format!("let {child_var} = {parent_var}.{accessor};"));
                for item in items {
                    self.generate_node_check(item, child_var, false);
                }
            }
            PatternNode::Ident(name) => {
                // An identifier as a child — probably a node type
                let cast = self
                    .mapping
                    .get(name.as_str())
                    .map(|m| m.cast_method.to_string());
                if let Some(cast) = cast {
                    self.writeln(&format!(
                        "if {parent_var}.{accessor}.{cast}().is_none() {{ return {fail}; }}"
                    ));
                } else {
                    self.writeln(&format!("// Identifier in child position: {name}"));
                }
            }
            PatternNode::IntLiteral(n) => {
                self.writeln(&format!(
                    "if {parent_var}.{accessor} != {n} {{ return {fail}; }}"
                ));
            }
            PatternNode::StringLiteral(s) => {
                self.writeln(&format!(
                    "if {parent_var}.{accessor} != b\"{s}\" {{ return {fail}; }}"
                ));
            }
            PatternNode::TypePredicate(typ) => {
                let cast = self
                    .mapping
                    .get(typ.as_str())
                    .map(|m| m.cast_method.to_string());
                if let Some(cast) = cast {
                    self.writeln(&format!(
                        "if {parent_var}.{accessor}.{cast}().is_none() {{ return {fail}; }}"
                    ));
                } else {
                    self.writeln(&format!("// Unknown type predicate: {typ}?"));
                }
            }
            _ => {
                self.writeln(&format!(
                    "// TODO: handle child pattern {:?}",
                    pattern_summary(child)
                ));
            }
        }
    }

    fn generate_alternatives(&mut self, alts: &[PatternNode], var: &str) {
        let fail = if self.has_captures { "None" } else { "false" };

        // Simple case: all alternatives are symbol literals (common for method name checks)
        let all_symbols = alts
            .iter()
            .all(|a| matches!(a, PatternNode::SymbolLiteral(_)));

        if all_symbols {
            let names: Vec<&str> = alts
                .iter()
                .filter_map(|a| match a {
                    PatternNode::SymbolLiteral(n) => Some(n.as_str()),
                    _ => None,
                })
                .collect();
            let conditions: Vec<String> =
                names.iter().map(|n| format!("{var} == b\"{n}\"")).collect();
            self.writeln(&format!("if !({})", conditions.join(" || ")));
            self.writeln(&format!("{{ return {fail}; }}"));
            return;
        }

        // Simple case: all alternatives are identifiers (node types)
        let all_idents = alts.iter().all(|a| matches!(a, PatternNode::Ident(_)));

        if all_idents {
            let mapping = &self.mapping;
            let checks: Vec<String> = alts
                .iter()
                .filter_map(|a| match a {
                    PatternNode::Ident(name) => mapping
                        .get(name.as_str())
                        .map(|m| format!("{var}.{}().is_some()", m.cast_method)),
                    _ => None,
                })
                .collect();
            if !checks.is_empty() {
                self.writeln(&format!("if !({})", checks.join(" || ")));
                self.writeln(&format!("{{ return {fail}; }}"));
            }
            return;
        }

        // General case: use a closure or match block
        self.writeln(&format!("// Alternatives check on {var}"));
        self.writeln("let _matched = {");
        self.indent += 1;
        self.writeln("let mut matched = false;");

        for (i, alt) in alts.iter().enumerate() {
            self.writeln(&format!("// Alternative {i}"));
            match alt {
                PatternNode::NodeMatch { node_type, .. } => {
                    if let Some(mapping) = self.mapping.get(node_type.as_str()) {
                        let cast = mapping.cast_method.to_string();
                        self.writeln(&format!("if let Some(_alt) = {var}.{cast}() {{"));
                        self.indent += 1;
                        self.writeln("matched = true;");
                        self.indent -= 1;
                        self.writeln("}");
                    }
                }
                PatternNode::HelperCall(name) => {
                    let fn_name = name.trim_end_matches('?');
                    self.writeln(&format!("if {fn_name}(&{var}) {{ matched = true; }}"));
                    if !self.helper_stubs.contains(&fn_name.to_string()) {
                        self.helper_stubs.push(fn_name.to_string());
                    }
                }
                PatternNode::NilPredicate => {
                    self.writeln(&format!("if {var}.is_none() {{ matched = true; }}"));
                }
                PatternNode::Ident(name) => {
                    let cast = self
                        .mapping
                        .get(name.as_str())
                        .map(|m| m.cast_method.to_string());
                    if let Some(cast) = cast {
                        self.writeln(&format!(
                            "if {var}.{cast}().is_some() {{ matched = true; }}"
                        ));
                    }
                }
                _ => {
                    self.writeln(&format!("// TODO: alternative {:?}", pattern_summary(alt)));
                }
            }
        }

        self.writeln("matched");
        self.indent -= 1;
        self.writeln("};");
        self.writeln(&format!("if !_matched {{ return {fail}; }}"));
    }

    fn generate_name_alternatives(
        &mut self,
        parent_var: &str,
        accessor: &str,
        alts: &[PatternNode],
    ) {
        let fail = if self.has_captures { "None" } else { "false" };
        let names: Vec<String> = alts
            .iter()
            .filter_map(|a| match a {
                PatternNode::SymbolLiteral(n) => Some(n.clone()),
                PatternNode::Ident(n) => Some(n.clone()),
                _ => None,
            })
            .collect();

        if names.len() == alts.len() {
            let conditions: Vec<String> = names
                .iter()
                .map(|n| format!("{parent_var}.{accessor} == b\"{n}\""))
                .collect();
            self.writeln(&format!("if !({})", conditions.join(" || ")));
            self.writeln(&format!("{{ return {fail}; }}"));
        } else {
            self.writeln(&format!(
                "// Complex alternatives on {parent_var}.{accessor} — manual review needed"
            ));
        }
    }

    fn generate_negation(&mut self, inner: &PatternNode, var: &str) {
        let fail = if self.has_captures { "None" } else { "false" };

        match inner {
            PatternNode::NodeMatch { node_type, .. } => {
                let cast = self
                    .mapping
                    .get(node_type.as_str())
                    .map(|m| m.cast_method.to_string());
                if let Some(cast) = cast {
                    self.writeln(&format!("if {var}.{cast}().is_some() {{ return {fail}; }}"));
                } else {
                    self.writeln(&format!("// Negation of unmapped type: {node_type}"));
                }
            }
            PatternNode::HelperCall(name) => {
                let fn_name = name.trim_end_matches('?');
                self.writeln(&format!("if {fn_name}(&{var}) {{ return {fail}; }}"));
                if !self.helper_stubs.contains(&fn_name.to_string()) {
                    self.helper_stubs.push(fn_name.to_string());
                }
            }
            _ => {
                self.writeln(&format!(
                    "// TODO: negation of {:?}",
                    pattern_summary(inner)
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

fn print_usage() {
    eprintln!("Usage: node_pattern_codegen <command> [args]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  generate <ruby_file>              Parse Ruby cop file, output Rust matchers");
    eprintln!("  verify <ruby_file> <rust_file>     Compare generated vs existing Rust code");
    eprintln!(
        "  dump-patterns [--stats]            Walk vendor dirs, print patterns + parse status"
    );
}

fn cmd_generate(ruby_path: &str) -> io::Result<()> {
    let source = fs::read_to_string(ruby_path)?;
    let patterns = extract_patterns(&source);

    if patterns.is_empty() {
        eprintln!("No def_node_matcher or def_node_search patterns found in {ruby_path}");
        return Ok(());
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    writeln!(
        out,
        "// Auto-generated by node_pattern_codegen from {ruby_path}"
    )?;
    writeln!(out, "// Patterns extracted: {}", patterns.len())?;
    writeln!(out, "//")?;
    writeln!(
        out,
        "// WARNING: This is generated scaffolding. Review and adapt before use."
    )?;
    writeln!(out)?;

    let mut codegen = CodeGenerator::new();

    for extracted in &patterns {
        let kind_label = match extracted.kind {
            PatternKind::Matcher => "def_node_matcher",
            PatternKind::Search => "def_node_search",
        };

        writeln!(out, "// --- {kind_label} :{} ---", extracted.method_name)?;
        writeln!(out, "// Pattern: {}", extracted.pattern.replace('\n', " "))?;

        // Lex
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();

        // Parse
        let mut parser = Parser::new(tokens);
        let Some(ast) = parser.parse() else {
            writeln!(
                out,
                "// ERROR: Failed to parse pattern for :{}",
                extracted.method_name
            )?;
            writeln!(out)?;
            continue;
        };

        // Generate
        let code = codegen.generate_pattern(extracted, &ast);
        writeln!(out, "{code}")?;

        // For search patterns, add a note
        if extracted.kind == PatternKind::Search {
            writeln!(
                out,
                "// NOTE: def_node_search yields all matching descendants."
            )?;
            writeln!(
                out,
                "// Wrap the above function in a tree-walk to search all nodes."
            )?;
            writeln!(out)?;
        }
    }

    Ok(())
}

fn cmd_verify(ruby_path: &str, rust_path: &str) {
    eprintln!("verify mode not yet implemented");
    eprintln!("  Ruby file: {ruby_path}");
    eprintln!("  Rust file: {rust_path}");
}

fn cmd_dump_patterns(stats_only: bool) {
    use std::collections::HashMap;
    use std::path::PathBuf;

    let vendor_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor");
    if !vendor_root.is_dir() {
        eprintln!("vendor/ directory not found. Run: git submodule update --init");
        process::exit(1);
    }

    let patterns = walk_vendor_patterns(&vendor_root);
    if patterns.is_empty() {
        eprintln!("No patterns extracted. Are vendor submodules initialized?");
        process::exit(1);
    }

    let mut total = 0usize;
    let mut parse_ok = 0usize;
    let mut parse_fail = 0usize;
    let mut gem_stats: HashMap<String, (usize, usize)> = HashMap::new();
    let mut dept_stats: HashMap<String, (usize, usize)> = HashMap::new();

    for (cop_name, extracted) in &patterns {
        total += 1;

        let dept = cop_name.split('/').next().unwrap_or("Unknown").to_string();

        // Infer gem from department
        let gem = match dept.as_str() {
            "Rails" => "rubocop-rails",
            "RSpec" => "rubocop-rspec",
            "RSpecRails" => "rubocop-rspec_rails",
            "Performance" => "rubocop-performance",
            "FactoryBot" => "rubocop-factory_bot",
            _ => "rubocop",
        }
        .to_string();

        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let parsed = parser.parse().is_some();

        let gem_entry = gem_stats.entry(gem).or_insert((0, 0));
        let dept_entry = dept_stats.entry(dept.clone()).or_insert((0, 0));

        if parsed {
            parse_ok += 1;
            gem_entry.0 += 1;
            dept_entry.0 += 1;
        } else {
            parse_fail += 1;
            gem_entry.1 += 1;
            dept_entry.1 += 1;
        }

        if !stats_only {
            let kind_label = match extracted.kind {
                PatternKind::Matcher => "matcher",
                PatternKind::Search => "search",
            };
            let status = if parsed { "OK" } else { "FAIL" };
            let pattern_preview: String = extracted
                .pattern
                .replace('\n', " ")
                .chars()
                .take(80)
                .collect();
            println!(
                "{status:4}  {cop_name}::{} [{kind_label}] {pattern_preview}",
                extracted.method_name,
            );
        }
    }

    // Always print stats
    let rate = if total > 0 {
        (parse_ok as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    eprintln!();
    eprintln!("=== Pattern Parse Stats ===");
    eprintln!("Total:     {total}");
    eprintln!("Parse OK:  {parse_ok}");
    eprintln!("Parse FAIL:{parse_fail}");
    eprintln!("Rate:      {rate:.1}%");
    eprintln!();

    // Gem breakdown
    eprintln!("By gem:");
    let mut gems: Vec<_> = gem_stats.iter().collect();
    gems.sort_by_key(|(name, _)| (*name).clone());
    for (gem, (ok, fail)) in &gems {
        let t = ok + fail;
        let r = if t > 0 {
            (*ok as f64 / t as f64) * 100.0
        } else {
            0.0
        };
        eprintln!("  {gem:30} {ok:4}/{t:4} ({r:.0}%)");
    }
    eprintln!();

    // Department breakdown
    eprintln!("By department:");
    let mut depts: Vec<_> = dept_stats.iter().collect();
    depts.sort_by_key(|(name, _)| (*name).clone());
    for (dept, (ok, fail)) in &depts {
        let t = ok + fail;
        let r = if t > 0 {
            (*ok as f64 / t as f64) * 100.0
        } else {
            0.0
        };
        eprintln!("  {dept:20} {ok:4}/{t:4} ({r:.0}%)");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let command = &args[1];
    match command.as_str() {
        "generate" => {
            if args.len() < 3 {
                eprintln!("generate requires a <ruby_file> argument");
                print_usage();
                process::exit(1);
            }
            let ruby_path = &args[2];
            if let Err(e) = cmd_generate(ruby_path) {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
        "verify" => {
            if args.len() < 4 {
                eprintln!("verify requires both <ruby_file> and <rust_file>");
                print_usage();
                process::exit(1);
            }
            cmd_verify(&args[2], &args[3]);
        }
        "dump-patterns" => {
            let stats_only = args.iter().any(|a| a == "--stats");
            cmd_dump_patterns(stats_only);
        }
        _ => {
            eprintln!("Unknown command: {command}");
            print_usage();
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codegen_simple_bool() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "where_method?".to_string(),
            pattern: "(send _ :where ...)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("fn where_method("));
        assert!(code.contains("-> bool"));
        assert!(code.contains("as_call_node"));
        assert!(code.contains("b\"where\""));
    }

    #[test]
    fn test_codegen_with_capture() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "find_method".to_string(),
            pattern: "(send _ ${:first :take})".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("fn find_method"));
        assert!(code.contains("Option"));
    }

    #[test]
    fn test_codegen_nil_predicate() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "explicit_receiver?".to_string(),
            pattern: "(send nil? :foo)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("fn explicit_receiver("));
        assert!(code.contains("as_call_node"));
        assert!(code.contains("is_some()")); // nil? -> receiver is_some check
        assert!(code.contains("b\"foo\""));
    }

    #[test]
    fn test_codegen_helper_generates_stub() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "match_with_helper?".to_string(),
            pattern: "(send #flow_command? _ ...)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(
            code.contains("fn match_with_helper("),
            "Should generate main function"
        );
        assert!(
            code.contains("flow_command"),
            "Should reference helper function"
        );
        assert!(
            code.contains("Generated stub"),
            "Should generate helper stub"
        );
        assert!(code.contains("todo!"), "Stub should have todo!()");
    }

    #[test]
    fn test_codegen_block_node() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "block_pass?".to_string(),
            pattern: "(block _ (args) _)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("as_block_node"), "Should cast to BlockNode");
    }

    #[test]
    fn test_codegen_def_node() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "method_def?".to_string(),
            pattern: "(def :initialize ...)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("as_def_node"), "Should cast to DefNode");
        assert!(code.contains("b\"initialize\""), "Should check method name");
    }

    #[test]
    fn test_codegen_alternatives_symbols() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "accessor?".to_string(),
            pattern: "(send nil? {:attr_reader :attr_writer :attr_accessor} ...)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(
            code.contains("b\"attr_reader\""),
            "Should check attr_reader"
        );
        assert!(
            code.contains("b\"attr_writer\""),
            "Should check attr_writer"
        );
        assert!(
            code.contains("b\"attr_accessor\""),
            "Should check attr_accessor"
        );
        assert!(code.contains("||"), "Should use OR for alternatives");
    }

    #[test]
    fn test_codegen_negation_helper() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "not_lazy?".to_string(),
            pattern: "(send !#lazy? _ ...)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("as_call_node"), "Should cast to CallNode");
        assert!(
            code.contains("lazy"),
            "Should reference the helper in negation"
        );
    }

    #[test]
    fn test_codegen_negation_node_match() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "not_send?".to_string(),
            pattern: "(block !(send nil? :skip ...) _ _)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("as_block_node"), "Should cast to BlockNode");
        assert!(
            code.contains("as_call_node"),
            "Should reference CallNode for negated send"
        );
    }

    #[test]
    fn test_e2e_extract_lex_parse_generate() {
        let source = r#"
        def_node_matcher :where_take?, <<~PATTERN
          (send (send _ :where ...) {:first :take})
        PATTERN
        "#;

        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);

        let pattern = &patterns[0];
        let mut lexer = Lexer::new(&pattern.pattern);
        let tokens = lexer.tokenize();
        assert!(!tokens.is_empty(), "Lexer should produce tokens");

        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parser should produce AST");

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(pattern, &ast);

        assert!(
            code.contains("fn where_take("),
            "Function name should be where_take"
        );
        assert!(
            code.contains("as_call_node"),
            "Should cast to CallNode for send"
        );
        assert!(
            code.contains("b\"where\""),
            "Should check for :where method name"
        );
        assert!(
            code.contains("b\"first\""),
            "Should check for :first alternative"
        );
        assert!(
            code.contains("b\"take\""),
            "Should check for :take alternative"
        );
    }

    #[test]
    fn test_e2e_rspec_expect_pattern() {
        let source = "def_node_matcher :expect?, '(send nil? :expect ...)'";
        let patterns = extract_patterns(source);
        let pattern = &patterns[0];

        let mut lexer = Lexer::new(&pattern.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(pattern, &ast);

        // Should generate a bool-returning function
        assert!(code.contains("-> bool"));
        assert!(code.contains("as_call_node"));
        // nil? should check receiver is None
        assert!(code.contains("is_some()"));
        assert!(code.contains("b\"expect\""));
    }

    #[test]
    fn test_count_captures_simple() {
        let ast = PatternNode::NodeMatch {
            node_type: "send".to_string(),
            children: vec![
                PatternNode::Wildcard,
                PatternNode::Capture(Box::new(PatternNode::SymbolLiteral("foo".to_string()))),
                PatternNode::Rest,
            ],
        };
        assert_eq!(CodeGenerator::count_captures(&ast), 1);
    }

    #[test]
    fn test_count_captures_none() {
        let ast = PatternNode::NodeMatch {
            node_type: "send".to_string(),
            children: vec![PatternNode::Wildcard, PatternNode::Rest],
        };
        assert_eq!(CodeGenerator::count_captures(&ast), 0);
    }

    #[test]
    fn test_count_captures_nested() {
        let ast = PatternNode::NodeMatch {
            node_type: "send".to_string(),
            children: vec![
                PatternNode::Capture(Box::new(PatternNode::Wildcard)),
                PatternNode::Capture(Box::new(PatternNode::SymbolLiteral("x".to_string()))),
            ],
        };
        assert_eq!(CodeGenerator::count_captures(&ast), 2);
    }
}
