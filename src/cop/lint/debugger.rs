use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct Debugger;

impl Cop for Debugger {
    fn name(&self) -> &'static str {
        "Lint/Debugger"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        let is_debugger = match method_name {
            b"pry" | b"remote_pry" | b"pry_remote" => {
                // binding.pry, binding.remote_pry, binding.pry_remote
                if let Some(recv) = call.receiver() {
                    if let Some(recv_call) = recv.as_call_node() {
                        recv_call.name().as_slice() == b"binding"
                            && recv_call.receiver().is_none()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            b"debugger" | b"byebug" => {
                // receiver-less debugger/byebug calls
                call.receiver().is_none()
            }
            b"irb" => {
                // binding.irb
                if let Some(recv) = call.receiver() {
                    if let Some(recv_call) = recv.as_call_node() {
                        recv_call.name().as_slice() == b"binding"
                            && recv_call.receiver().is_none()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        };

        if is_debugger {
            let loc = call.location();
            let source_text = std::str::from_utf8(loc.as_slice()).unwrap_or("debugger");
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: self.default_severity(),
                cop_name: self.name().to_string(),
                message: format!("Remove debugger entry point `{source_text}`."),
            }]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &Debugger,
            include_bytes!("../../../testdata/cops/lint/debugger/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &Debugger,
            include_bytes!("../../../testdata/cops/lint/debugger/no_offense.rb"),
        );
    }
}
