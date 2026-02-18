use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MixinGrouping;

const MIXIN_METHODS: &[&[u8]] = &[b"include", b"extend", b"prepend"];

impl Cop for MixinGrouping {
    fn name(&self) -> &'static str {
        "Style/MixinGrouping"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "separated");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        if !MIXIN_METHODS.iter().any(|&m| m == method_bytes) {
            return Vec::new();
        }

        // Must not have a receiver (bare include/extend/prepend)
        if call.receiver().is_some() {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();

        if style == "separated" && arg_list.len() > 1 {
            let method_str = std::str::from_utf8(method_bytes).unwrap_or("include");
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Put `{method_str}` mixins in separate statements."),
            )];
        }

        if style == "grouped" && arg_list.len() == 1 {
            // For grouped style, we'd need to detect consecutive single-arg mixin calls
            // which is complex. For now, only flag the separated style.
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MixinGrouping, "cops/style/mixin_grouping");
}
