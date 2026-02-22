use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AmbiguousEndlessMethodDefinition;

impl Cop for AmbiguousEndlessMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/AmbiguousEndlessMethodDefinition"
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let low_precedence_ops = [" and ", " or ", " if ", " unless ", " while ", " until "];

        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s.trim_end(),
                Err(_) => continue,
            };

            // Check for endless method definition: `def foo = ...`
            let trimmed = line_str.trim_start();
            if !trimmed.starts_with("def ") {
                continue;
            }

            // Find the `=` that makes it endless
            // Look for `= ` after method name (not `==`)
            let after_def = &trimmed[4..];
            let eq_pos = after_def.find(" = ");
            if eq_pos.is_none() {
                continue;
            }

            let eq_pos = eq_pos.unwrap();
            let after_eq = &after_def[eq_pos + 3..];

            // Check if there's a low-precedence operator in the body
            // that isn't wrapped in parentheses
            for op in &low_precedence_ops {
                if after_eq.contains(op) {
                    // Check it's not inside parentheses
                    let op_pos = after_eq.find(op).unwrap();
                    let before_op = &after_eq[..op_pos];
                    let paren_depth: i32 = before_op
                        .chars()
                        .map(|c| match c {
                            '(' => 1,
                            ')' => -1,
                            _ => 0,
                        })
                        .sum();

                    if paren_depth == 0 {
                        let col = 0;
                        let op_name = op.trim();
                        diagnostics.push(self.diagnostic(
                            source,
                            i + 1,
                            col,
                            format!("Avoid using `{}` statements with endless methods.", op_name),
                        ));
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        AmbiguousEndlessMethodDefinition,
        "cops/style/ambiguous_endless_method_definition"
    );
}
