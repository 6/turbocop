use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Pending;

/// x-prefixed methods that indicate pending specs.
const XMETHODS: &[&[u8]] = &[
    b"xcontext", b"xdescribe", b"xexample", b"xfeature",
    b"xit", b"xscenario", b"xspecify",
];

impl Cop for Pending {
    fn name(&self) -> &'static str {
        "RSpec/Pending"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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

        // Check for x-prefixed methods with blocks
        if XMETHODS.contains(&method_name) && call.receiver().is_none() && call.block().is_some() {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(source, line, column, "Pending spec found.".to_string())];
        }

        // Check for `pending 'test' do` and `skip 'test' do` as example methods
        if (method_name == b"pending" || method_name == b"skip")
            && call.receiver().is_none()
            && call.block().is_some()
        {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(source, line, column, "Pending spec found.".to_string())];
        }

        // Check for `skip` without arguments inside an example (standalone call)
        if method_name == b"skip"
            && call.receiver().is_none()
            && call.arguments().is_none()
            && call.block().is_none()
        {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(source, line, column, "Pending spec found.".to_string())];
        }

        // Check for `it 'test'` without a block (pending example)
        let example_methods: &[&[u8]] = &[
            b"it", b"specify", b"example", b"scenario",
        ];
        if example_methods.contains(&method_name)
            && call.receiver().is_none()
            && call.block().is_none()
            && call.arguments().is_some()
        {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(source, line, column, "Pending spec found.".to_string())];
        }

        // Check for :skip or :pending metadata, or skip: true/string, pending: true/string
        if call.receiver().is_none() || {
            if let Some(recv) = call.receiver() {
                recv.as_constant_read_node()
                    .map(|c| c.name().as_slice() == b"RSpec")
                    .unwrap_or(false)
            } else {
                false
            }
        } {
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    // Check for :skip or :pending symbol metadata
                    if let Some(sym) = arg.as_symbol_node() {
                        let val = sym.unescaped();
                        if val == b"skip" || val == b"pending" {
                            let loc = call.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                "Pending spec found.".to_string(),
                            )];
                        }
                    }
                    // Check for skip: true/string, pending: true/string in keyword args
                    if let Some(kw) = arg.as_keyword_hash_node() {
                        for elem in kw.elements().iter() {
                            if let Some(assoc) = elem.as_assoc_node() {
                                if let Some(key_sym) = assoc.key().as_symbol_node() {
                                    let key = key_sym.unescaped();
                                    if key == b"skip" || key == b"pending" {
                                        // skip: false is not pending
                                        if assoc.value().as_false_node().is_some() {
                                            continue;
                                        }
                                        let loc = call.location();
                                        let (line, column) =
                                            source.offset_to_line_col(loc.start_offset());
                                        return vec![self.diagnostic(
                                            source,
                                            line,
                                            column,
                                            "Pending spec found.".to_string(),
                                        )];
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Pending, "cops/rspec/pending");
}
