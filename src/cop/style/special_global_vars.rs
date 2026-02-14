use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct SpecialGlobalVars;

fn perl_to_english(name: &[u8]) -> Option<&'static str> {
    match name {
        b"$!" => Some("$ERROR_INFO"),
        b"$@" => Some("$ERROR_POSITION"),
        b"$;" => Some("$FIELD_SEPARATOR"),
        b"$," => Some("$OUTPUT_FIELD_SEPARATOR"),
        b"$/" => Some("$INPUT_RECORD_SEPARATOR"),
        b"$\\" => Some("$OUTPUT_RECORD_SEPARATOR"),
        b"$." => Some("$INPUT_LINE_NUMBER"),
        b"$0" => Some("$PROGRAM_NAME"),
        b"$$" => Some("$PROCESS_ID"),
        b"$?" => Some("$CHILD_STATUS"),
        b"$~" => Some("$LAST_MATCH_INFO"),
        b"$&" => Some("$MATCH"),
        b"$'" => Some("$POSTMATCH"),
        b"$`" => Some("$PREMATCH"),
        b"$+" => Some("$LAST_PAREN_MATCH"),
        b"$_" => Some("$LAST_READ_LINE"),
        b"$>" => Some("$DEFAULT_OUTPUT"),
        b"$<" => Some("$DEFAULT_INPUT"),
        b"$*" => Some("$ARGV"),
        _ => None,
    }
}

impl Cop for SpecialGlobalVars {
    fn name(&self) -> &'static str {
        "Style/SpecialGlobalVars"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let gvar = match node.as_global_variable_read_node() {
            Some(g) => g,
            None => return Vec::new(),
        };

        let loc = gvar.location();
        let var_name = loc.as_slice();

        if let Some(english) = perl_to_english(var_name) {
            let perl_name = std::str::from_utf8(var_name).unwrap_or("$?");
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: format!("Prefer `{}` over `{}`.", english, perl_name),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &SpecialGlobalVars,
            include_bytes!("../../../testdata/cops/style/special_global_vars/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &SpecialGlobalVars,
            include_bytes!("../../../testdata/cops/style/special_global_vars/no_offense.rb"),
        );
    }

    #[test]
    fn regular_global_is_ignored() {
        let source = b"x = $foo\n";
        let diags = run_cop_full(&SpecialGlobalVars, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn multiple_perl_vars_all_flagged() {
        let source = b"puts $!\nputs $$\n";
        let diags = run_cop_full(&SpecialGlobalVars, source);
        assert_eq!(diags.len(), 2);
    }
}
