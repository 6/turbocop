use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UniqueValidationWithoutIndex;

impl Cop for UniqueValidationWithoutIndex {
    fn name(&self) -> &'static str {
        "Rails/UniqueValidationWithoutIndex"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        // Only receiverless calls (DSL-style)
        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();

        if method_name == b"validates" {
            return self.check_validates(source, &call);
        }

        if method_name == b"validates_uniqueness_of" {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Uniqueness validation should have a unique index on the database column."
                    .to_string(),
            )];
        }

        Vec::new()
    }
}

impl UniqueValidationWithoutIndex {
    fn check_validates(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
    ) -> Vec<Diagnostic> {
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Look for `uniqueness: true` or `uniqueness: { ... }` in keyword args
        for arg in args.arguments().iter() {
            if let Some(kw_hash) = arg.as_keyword_hash_node() {
                if self.has_uniqueness_key(&kw_hash) {
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Uniqueness validation should have a unique index on the database column."
                            .to_string(),
                    )];
                }
            }
            if let Some(hash) = arg.as_hash_node() {
                if self.has_uniqueness_key_in_hash(&hash) {
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Uniqueness validation should have a unique index on the database column."
                            .to_string(),
                    )];
                }
            }
        }

        Vec::new()
    }

    fn has_uniqueness_key(&self, kw_hash: &ruby_prism::KeywordHashNode<'_>) -> bool {
        for elem in kw_hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if let Some(sym) = assoc.key().as_symbol_node() {
                    if sym.unescaped() == b"uniqueness" {
                        // Check the value is not `false` or `nil`
                        let value = assoc.value();
                        if value.as_false_node().is_some() || value.as_nil_node().is_some() {
                            return false;
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    fn has_uniqueness_key_in_hash(&self, hash: &ruby_prism::HashNode<'_>) -> bool {
        for elem in hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if let Some(sym) = assoc.key().as_symbol_node() {
                    if sym.unescaped() == b"uniqueness" {
                        let value = assoc.value();
                        if value.as_false_node().is_some() || value.as_nil_node().is_some() {
                            return false;
                        }
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UniqueValidationWithoutIndex,
        "cops/rails/unique_validation_without_index"
    );
}
