use crate::cop::shared::node_type::{CLASS_NODE, MODULE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for class and module names with underscores.
///
/// ## Investigation findings (FN=13)
///
/// **Root cause:** Nitrocop was only checking the rightmost segment of qualified
/// constant paths (e.g., for `module My_Module::Ala`, it checked only `Ala`).
/// RuboCop checks the FULL source text of the name (`My_Module::Ala`) for
/// underscores.
///
/// **Fix:** Extract the full source text from the name's location span, check
/// for underscores after stripping AllowedNames patterns (regex gsub matching
/// RuboCop behavior). Report offense at the full name location.
///
/// **AllowedNames handling:** RuboCop builds a regex from the AllowedNames array
/// (default: `['module_parent']`) and gsub-strips matching substrings from the
/// full name source. If underscores remain after stripping, it's an offense.
/// This allows `module_parent::MyClass` to pass (the underscore is in an
/// allowed name).
///
/// Follow-up (2026-03-08): FP=1 regressed at a site using
/// `# rubocop:disable Style/ClassAndModuleCamelCase`. RuboCop still suppresses
/// `Naming/ClassAndModuleCamelCase` for that moved legacy name because the
/// short name stayed `ClassAndModuleCamelCase`. Fixed centrally in
/// `parse/directives.rs`.
pub struct ClassAndModuleCamelCase;

impl Cop for ClassAndModuleCamelCase {
    fn name(&self) -> &'static str {
        "Naming/ClassAndModuleCamelCase"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, MODULE_NODE]
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
        let name_node = if let Some(class_node) = node.as_class_node() {
            class_node.constant_path()
        } else if let Some(module_node) = node.as_module_node() {
            module_node.constant_path()
        } else {
            return;
        };

        // Get the full source text of the name (e.g., "Top::My_Class", "My_Module::Ala")
        let loc = name_node.location();
        let start = loc.start_offset();
        let end = loc.end_offset();
        let name_source = match source.try_byte_slice(start, end) {
            Some(s) => s,
            None => return,
        };

        // Quick check: if no underscore in the full name, it's fine
        if !name_source.contains('_') {
            return;
        }

        // Strip AllowedNames patterns from the full name (RuboCop does regex gsub).
        // Default: ['module_parent'] per vendor config/default.yml.
        let default_allowed = vec!["module_parent".to_string()];
        let allowed_names = config
            .get_string_array("AllowedNames")
            .unwrap_or(default_allowed);
        let mut cleaned = name_source.to_string();
        for pattern in &allowed_names {
            cleaned = cleaned.replace(pattern.as_str(), "");
        }

        // After stripping allowed names, if no underscore remains, it's allowed
        if !cleaned.contains('_') {
            return;
        }

        let (line, column) = source.offset_to_line_col(start);

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use CamelCase for classes and modules.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ClassAndModuleCamelCase,
        "cops/naming/class_and_module_camel_case"
    );
}
