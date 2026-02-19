use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, DEF_NODE, SYMBOL_NODE};

pub struct PredicatePrefix;

impl PredicatePrefix {
    fn check_method_name(
        &self,
        source: &SourceFile,
        name_str: &str,
        name_offset: usize,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // NamePrefix identifies which prefixes mark a method as a predicate.
        // ForbiddenPrefixes is the subset that should be removed.
        let _name_prefixes = config
            .get_string_array("NamePrefix")
            .unwrap_or_else(|| vec!["is_".into(), "has_".into(), "have_".into(), "does_".into()]);

        let forbidden_prefixes = config
            .get_string_array("ForbiddenPrefixes")
            .unwrap_or_else(|| vec!["is_".into(), "has_".into(), "have_".into(), "does_".into()]);

        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .unwrap_or_else(|| vec!["is_a?".into()]);

        // UseSorbetSigs: when true, only flag methods with T::Boolean return sigs.
        // We don't support Sorbet sig analysis, so when enabled we skip all checks
        // (conservative: no false positives).
        let use_sorbet_sigs = config.get_bool("UseSorbetSigs", false);
        if use_sorbet_sigs {
            return Vec::new();
        }

        if allowed_methods.iter().any(|m| m == name_str) {
            return Vec::new();
        }

        let matching_prefix = forbidden_prefixes
            .iter()
            .find(|p| name_str.starts_with(p.as_str()));
        let prefix = match matching_prefix {
            Some(p) => p,
            None => return Vec::new(),
        };

        let suggested = &name_str[prefix.len()..];
        let (line, column) = source.offset_to_line_col(name_offset);

        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Rename `{name_str}` to `{suggested}`."),
        )]
    }
}

impl Cop for PredicatePrefix {
    fn name(&self) -> &'static str {
        "Naming/PredicatePrefix"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, DEF_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Handle regular def nodes
        if let Some(def_node) = node.as_def_node() {
            let method_name = def_node.name().as_slice();
            let name_str = match std::str::from_utf8(method_name) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };
            return self.check_method_name(source, name_str, def_node.name_loc().start_offset(), config);
        }

        // Handle MethodDefinitionMacros (e.g. define_method(:is_even))
        if let Some(call_node) = node.as_call_node() {
            let macros = config
                .get_string_array("MethodDefinitionMacros")
                .unwrap_or_else(|| {
                    vec![
                        "define_method".into(),
                        "define_singleton_method".into(),
                    ]
                });

            let call_name = call_node.name().as_slice();
            let call_name_str = match std::str::from_utf8(call_name) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };

            if !macros.iter().any(|m| m == call_name_str) {
                return Vec::new();
            }

            // First argument should be a symbol literal with the method name
            let args = match call_node.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };
            let args_list: Vec<_> = args.arguments().iter().collect();
            if args_list.is_empty() {
                return Vec::new();
            }
            if let Some(sym) = args_list[0].as_symbol_node() {
                let sym_bytes = sym.unescaped();
                let sym_str = match std::str::from_utf8(&sym_bytes) {
                    Ok(s) => s,
                    Err(_) => return Vec::new(),
                };
                return self.check_method_name(
                    source,
                    sym_str,
                    sym.location().start_offset(),
                    config,
                );
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PredicatePrefix, "cops/naming/predicate_prefix");
}
