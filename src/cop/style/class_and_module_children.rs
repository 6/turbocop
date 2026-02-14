use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct ClassAndModuleChildren;

impl Cop for ClassAndModuleChildren {
    fn name(&self) -> &'static str {
        "Style/ClassAndModuleChildren"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config
            .options
            .get("EnforcedStyle")
            .and_then(|v| v.as_str())
            .unwrap_or("nested");

        if let Some(class_node) = node.as_class_node() {
            let constant_path = class_node.constant_path();
            let is_compact = constant_path.as_constant_path_node().is_some();

            if enforced_style == "nested" && is_compact {
                let kw_loc = class_node.class_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Use nested module/class definitions instead of compact style."
                        .to_string(),
                }];
            } else if enforced_style == "compact" && !is_compact {
                let kw_loc = class_node.class_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Use compact module/class definition instead of nested style."
                        .to_string(),
                }];
            }
        } else if let Some(module_node) = node.as_module_node() {
            let constant_path = module_node.constant_path();
            let is_compact = constant_path.as_constant_path_node().is_some();

            if enforced_style == "nested" && is_compact {
                let kw_loc = module_node.module_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Use nested module/class definitions instead of compact style."
                        .to_string(),
                }];
            } else if enforced_style == "compact" && !is_compact {
                let kw_loc = module_node.module_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Use compact module/class definition instead of nested style."
                        .to_string(),
                }];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &ClassAndModuleChildren,
            include_bytes!(
                "../../../testdata/cops/style/class_and_module_children/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &ClassAndModuleChildren,
            include_bytes!(
                "../../../testdata/cops/style/class_and_module_children/no_offense.rb"
            ),
        );
    }
}
