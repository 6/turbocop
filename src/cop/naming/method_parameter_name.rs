use crate::cop::shared::node_type::{
    DEF_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, OPTIONAL_PARAMETER_NODE,
    REQUIRED_KEYWORD_PARAMETER_NODE, REQUIRED_PARAMETER_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MethodParameterName;

/// ## Corpus investigation (2026-03-02)
///
/// Corpus oracle run #28 reported FP=33, FN=388. Investigation found that the
/// 33 FP were entirely within file-drop noise from 1 repo (jruby) with RuboCop
/// parser crashes — RuboCop under-counted offenses in that repo because it
/// couldn't parse some files. Running both tools directly on the affected repos
/// (samg/timetrap, rubyworks/facets) with the baseline config produced identical
/// offense counts (0 for both tools). check-cop.py confirms: Excess=0 after
/// adjusting for file-drop noise of 1,712. No implementation fix needed.
///
/// ## FN=358 fix (2026-03-08)
///
/// Root causes of false negatives:
/// 1. Missing uppercase/camelCase check — RuboCop's UncommunicativeName mixin
///    flags any param containing uppercase chars. Nitrocop had zero implementation.
/// 2. `_`-prefixed params skipped entirely — RuboCop strips leading underscores
///    and checks the basename. Nitrocop returned early for all `_`-prefixed names.
/// 3. `ForbiddenNames` config read but unused (underscore-prefixed variable).
/// 4. `AllowNamesEndingInNumbers` config read but unused.
///
/// Fix: strip leading underscores to get basename, add uppercase check, wire up
/// ForbiddenNames and AllowNamesEndingInNumbers.
///
/// ## FN=12 fix (2026-03-08)
///
/// All 12 FNs were post-splat required parameters (e.g., `def m(*args, a, b)`).
/// These live in `params.posts()` in Prism's AST, which was not iterated.
/// Fix: add `params.posts()` iteration alongside `params.requireds()`.
///
/// ## CI check-cop regression (2026-03-24) — standard corpus, +7 FP
///
/// check-cop reports 11,960 vs 11,952 expected (+8 excess, gate says +7 FP
/// after file-drop adjustment). Triggered when naming-extended branch doc
/// comments touched this file, causing CI to re-check the cop. The naming
/// commits were reverted pending investigation. The oracle lacks
/// `nitro_total_unfiltered` for this cop, so check-cop compares against the
/// filtered RuboCop count — the 8 extra offenses may be on files where
/// RuboCop crashed but nitrocop parsed successfully. However, this was not
/// conclusively proven. Before re-landing naming-extended changes, either:
/// (a) update the corpus oracle to emit `nitro_total_unfiltered`, or
/// (b) identify the exact 8 excess offenses and confirm they are benign.
const DEFAULT_ALLOWED: &[&str] = &[
    "as", "at", "by", "cc", "db", "id", "if", "in", "io", "ip", "of", "on", "os", "pp", "to",
];

impl Cop for MethodParameterName {
    fn name(&self) -> &'static str {
        "Naming/MethodParameterName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            DEF_NODE,
            OPTIONAL_KEYWORD_PARAMETER_NODE,
            OPTIONAL_PARAMETER_NODE,
            REQUIRED_KEYWORD_PARAMETER_NODE,
            REQUIRED_PARAMETER_NODE,
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
        let min_length = config.get_usize("MinNameLength", 3);
        let allow_numbers = config.get_bool("AllowNamesEndingInNumbers", true);
        let allowed_names = config.get_string_array("AllowedNames");
        let forbidden_names = config.get_string_array("ForbiddenNames");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let allowed: Vec<String> = allowed_names
            .unwrap_or_else(|| DEFAULT_ALLOWED.iter().map(|s| s.to_string()).collect());
        let forbidden: Vec<String> = forbidden_names.unwrap_or_default();

        // Check required parameters
        for param in params.requireds().iter() {
            if let Some(req) = param.as_required_parameter_node() {
                let name = req.name().as_slice();
                check_param(
                    self,
                    source,
                    name,
                    &req.location(),
                    min_length,
                    &allowed,
                    &forbidden,
                    allow_numbers,
                    diagnostics,
                );
            }
        }

        // Check post-splat required parameters
        for param in params.posts().iter() {
            if let Some(req) = param.as_required_parameter_node() {
                let name = req.name().as_slice();
                check_param(
                    self,
                    source,
                    name,
                    &req.location(),
                    min_length,
                    &allowed,
                    &forbidden,
                    allow_numbers,
                    diagnostics,
                );
            }
        }

        // Check optional parameters
        for param in params.optionals().iter() {
            if let Some(opt) = param.as_optional_parameter_node() {
                let name = opt.name().as_slice();
                check_param(
                    self,
                    source,
                    name,
                    &opt.name_loc(),
                    min_length,
                    &allowed,
                    &forbidden,
                    allow_numbers,
                    diagnostics,
                );
            }
        }

        // Check keyword parameters
        for param in params.keywords().iter() {
            if let Some(kw) = param.as_required_keyword_parameter_node() {
                let name = kw.name().as_slice();
                // Strip trailing : from keyword name
                let clean_name = if name.ends_with(b":") {
                    &name[..name.len() - 1]
                } else {
                    name
                };
                check_param(
                    self,
                    source,
                    clean_name,
                    &kw.name_loc(),
                    min_length,
                    &allowed,
                    &forbidden,
                    allow_numbers,
                    diagnostics,
                );
            }
            if let Some(kw) = param.as_optional_keyword_parameter_node() {
                let name = kw.name().as_slice();
                let clean_name = if name.ends_with(b":") {
                    &name[..name.len() - 1]
                } else {
                    name
                };
                check_param(
                    self,
                    source,
                    clean_name,
                    &kw.name_loc(),
                    min_length,
                    &allowed,
                    &forbidden,
                    allow_numbers,
                    diagnostics,
                );
            }
        }

        // Check rest parameter (*args) — report at full node (including *) to match RuboCop
        if let Some(rest) = params.rest() {
            if let Some(rest_param) = rest.as_rest_parameter_node() {
                if rest_param.name_loc().is_some() {
                    let name = rest_param.name().map(|n| n.as_slice()).unwrap_or(b"");
                    check_param(
                        self,
                        source,
                        name,
                        &rest_param.location(),
                        min_length,
                        &allowed,
                        &forbidden,
                        allow_numbers,
                        diagnostics,
                    );
                }
            }
        }

        // Check keyword rest parameter (**kwargs) — report at full node (including **) to match RuboCop
        if let Some(kw_rest) = params.keyword_rest() {
            if let Some(kw_rest_param) = kw_rest.as_keyword_rest_parameter_node() {
                if kw_rest_param.name_loc().is_some() {
                    let name = kw_rest_param.name().map(|n| n.as_slice()).unwrap_or(b"");
                    check_param(
                        self,
                        source,
                        name,
                        &kw_rest_param.location(),
                        min_length,
                        &allowed,
                        &forbidden,
                        allow_numbers,
                        diagnostics,
                    );
                }
            }
        }

        // Check block parameter (&block) — report at full node (including &) to match RuboCop
        if let Some(block) = params.block() {
            if block.name_loc().is_some() {
                let name = block.name().map(|n| n.as_slice()).unwrap_or(b"");
                check_param(
                    self,
                    source,
                    name,
                    &block.location(),
                    min_length,
                    &allowed,
                    &forbidden,
                    allow_numbers,
                    diagnostics,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_param(
    cop: &MethodParameterName,
    source: &SourceFile,
    name: &[u8],
    loc: &ruby_prism::Location<'_>,
    min_length: usize,
    allowed: &[String],
    forbidden: &[String],
    allow_numbers: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name_str = std::str::from_utf8(name).unwrap_or("");

    // RuboCop only skips exactly `_` (single underscore). Double underscore `__`
    // still gets checked (basename is empty but length check fires).
    if name_str == "_" {
        return;
    }

    // Strip leading underscores to get basename (RuboCop checks the basename).
    let basename = name_str.trim_start_matches('_');

    // Check allowed names (against basename, matching RuboCop)
    if allowed.iter().any(|a| a == basename) {
        return;
    }

    let (line, column) = source.offset_to_line_col(loc.start_offset());

    // RuboCop emits multiple offenses per param: forbidden, case, length, numbers.
    // Check forbidden names
    if forbidden.iter().any(|f| f == basename) {
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Do not use `{basename}` as a name for a method parameter."),
        ));
    }

    // Check uppercase characters
    if basename.chars().any(|c| c.is_uppercase()) {
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            "Only use lowercase characters for method parameter.".to_string(),
        ));
    }

    // Check minimum length (against full name including underscores, matching RuboCop)
    if basename.len() < min_length {
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Method parameter must be at least {min_length} characters long."),
        ));
    }

    // Check names ending in numbers
    if !allow_numbers && basename.ends_with(|c: char| c.is_ascii_digit()) {
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            "Do not end method parameter with a number.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(MethodParameterName, "cops/naming/method_parameter_name");
}
