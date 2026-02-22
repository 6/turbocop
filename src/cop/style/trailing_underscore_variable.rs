use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{LOCAL_VARIABLE_TARGET_NODE, MULTI_WRITE_NODE, SPLAT_NODE};

pub struct TrailingUnderscoreVariable;

impl Cop for TrailingUnderscoreVariable {
    fn name(&self) -> &'static str {
        "Style/TrailingUnderscoreVariable"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[LOCAL_VARIABLE_TARGET_NODE, MULTI_WRITE_NODE, SPLAT_NODE]
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
        let allow_named = config.get_bool("AllowNamedUnderscoreVariables", true);

        let multi = match node.as_multi_write_node() {
            Some(m) => m,
            None => return,
        };

        let lefts: Vec<_> = multi.lefts().iter().collect();
        if lefts.is_empty() {
            return;
        }

        // Check trailing underscore variables
        let mut trailing_count = 0;
        for target in lefts.iter().rev() {
            if is_underscore_var(target, allow_named) {
                trailing_count += 1;
            } else {
                break;
            }
        }

        // Also check rest assignment
        if let Some(rest) = multi.rest() {
            if is_underscore_var(&rest, allow_named) && trailing_count == lefts.len() {
                trailing_count += 1;
            }
        }

        if trailing_count == 0 {
            return;
        }

        // Don't flag if ALL variables are underscores
        if trailing_count >= lefts.len() {
            return;
        }

        // RuboCop points at the first trailing underscore variable, not the whole assignment
        let first_trailing_idx = lefts.len() - trailing_count;
        let first_trailing = &lefts[first_trailing_idx];
        let loc = first_trailing.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Trailing underscore variable(s) in parallel assignment are unnecessary.".to_string(),
        ));
    }
}

fn is_underscore_var(node: &ruby_prism::Node<'_>, allow_named: bool) -> bool {
    if let Some(target) = node.as_local_variable_target_node() {
        let name = target.name().as_slice();
        if name == b"_" {
            return true;
        }
        if !allow_named && name.starts_with(b"_") {
            return true;
        }
        return false;
    }
    // Splat node like *_
    if let Some(splat) = node.as_splat_node() {
        if let Some(expr) = splat.expression() {
            if let Some(target) = expr.as_local_variable_target_node() {
                let name = target.name().as_slice();
                if name == b"_" {
                    return true;
                }
                if !allow_named && name.starts_with(b"_") {
                    return true;
                }
            }
        } else {
            // bare * (implicit rest)
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrailingUnderscoreVariable, "cops/style/trailing_underscore_variable");
}
