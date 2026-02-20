use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct ReverseFind;

impl Cop for ReverseFind {
    fn name(&self) -> &'static str {
        "Style/ReverseFind"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        // rfind is only available in Ruby >= 4.0
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_u64().map(|u| u as f64))
                    .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
            })
            .unwrap_or(2.7);
        if ruby_version < 4.0 {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be `.find` or `.detect`
        let name = call.name().as_slice();
        if name != b"find" && name != b"detect" {
            return;
        }

        // Receiver must be a `.reverse` call
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let recv_method = recv_call.name().as_slice();
        if recv_method != b"reverse" && recv_method != b"reverse_each" {
            return;
        }

        // `.reverse`/`.reverse_each` must have no arguments
        if recv_call.arguments().is_some() {
            return;
        }

        // Must have a block or block argument
        if call.block().is_none() {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `rfind` instead of `reverse.find`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use std::collections::HashMap;

    fn config_with_ruby(version: f64) -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(version)),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn offense_fixture() {
        let config = config_with_ruby(4.0);
        crate::testutil::assert_cop_offenses_full_with_config(
            &ReverseFind,
            include_bytes!("../../../testdata/cops/style/reverse_find/offense.rb"),
            config,
        );
    }

    #[test]
    fn no_offense_fixture() {
        let config = config_with_ruby(4.0);
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &ReverseFind,
            include_bytes!("../../../testdata/cops/style/reverse_find/no_offense.rb"),
            config,
        );
    }

    #[test]
    fn no_offense_when_ruby_below_4() {
        // On Ruby < 4.0, rfind doesn't exist, so nothing should be flagged
        let config = config_with_ruby(3.3);
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &ReverseFind,
            b"array.reverse.find { |x| x > 0 }",
            config,
        );
    }
}
