/// Checks for underscore-prefixed variables that are actually used.
///
/// RuboCop uses VariableForce to track variable scoping across all scope types
/// (def, block, lambda, top-level, class, module). This implementation uses the
/// shared VariableForce engine, which handles all scope tracking, parameter
/// shadowing, `super` forwarding, and `binding` implicit references.
///
/// Key behaviors matching RuboCop:
/// - Flags underscore-prefixed method params, block params, and local variable
///   assignments that are subsequently read in the same scope.
/// - Includes bare `_` — if `_` is used (read), it's an offense.
/// - Respects block parameter shadowing: if a block redefines a param with the
///   same name, reads inside the block are attributed to the block param, not
///   the outer scope variable.
/// - Handles `AllowKeywordBlockArguments` config to skip keyword block params.
/// - Skips variables implicitly forwarded via bare `super` or `binding`.
/// - Handles top-level scope, class/module bodies, and nested blocks.
/// - Handles destructured block parameters (e.g., `|(a, _b)|`).
/// - Handles pattern match variables (e.g., `case x; in _var; end`) including
///   guard clauses (`in _ if _.blank?`).
/// - Handles rescue exception captures (e.g., `rescue Error => _e`).
use crate::cop::variable_force::{self, DeclarationKind, Scope, VariableTable};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

use super::super::variable_force::scope::ScopeKind;
use super::super::variable_force::variable::Variable;

pub struct UnderscorePrefixedVariableName;

impl Cop for UnderscorePrefixedVariableName {
    fn name(&self) -> &'static str {
        "Lint/UnderscorePrefixedVariableName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn as_variable_force_consumer(&self) -> Option<&dyn variable_force::VariableForceConsumer> {
        Some(self)
    }
}

impl variable_force::VariableForceConsumer for UnderscorePrefixedVariableName {
    fn before_leaving_scope(
        &self,
        scope: &Scope,
        _variable_table: &VariableTable,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let allow_keyword_block_args = config.get_bool("AllowKeywordBlockArguments", false);
        let is_block_scope = matches!(scope.kind, ScopeKind::Block);

        for variable in scope.variables.values() {
            if let Some(diag) = check_variable(
                self,
                variable,
                allow_keyword_block_args,
                is_block_scope,
                source,
            ) {
                diagnostics.push(diag);
            }
        }
    }
}

fn check_variable(
    cop: &UnderscorePrefixedVariableName,
    variable: &Variable,
    allow_keyword_block_args: bool,
    is_block_scope: bool,
    source: &SourceFile,
) -> Option<Diagnostic> {
    // Only check variables whose name starts with `_`
    if !variable.should_be_unused() {
        return None;
    }

    // Skip keyword block arguments when AllowKeywordBlockArguments is true
    if allow_keyword_block_args
        && is_block_scope
        && matches!(
            variable.declaration_kind,
            DeclarationKind::KeywordArg | DeclarationKind::OptionalKeywordArg
        )
    {
        return None;
    }

    // Check if variable has any explicit references.
    // Implicit references (from `super` or `binding`) don't count for this cop.
    let has_explicit_ref = variable.references.iter().any(|r| r.explicit);

    if !has_explicit_ref {
        return None;
    }

    let (line, column) = source.offset_to_line_col(variable.declaration_offset);
    Some(cop.diagnostic(
        source,
        line,
        column,
        "Do not use prefix `_` for a variable that is used.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UnderscorePrefixedVariableName,
        "cops/lint/underscore_prefixed_variable_name"
    );

    #[test]
    fn test_block_param_used_in_method_call() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"def foo\n  proxy = @proxies.detect do |_proxy|\n    _proxy.params.has_key?(param_key)\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _proxy, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_local_var_in_block_used() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"def foo\n  items.each do |item|\n    _val = item.process\n    puts _val\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _val, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_bare_underscore_used() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"items.each { |_| _ }\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for bare _, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_no_double_report_outer_reassignment() {
        let cop = UnderscorePrefixedVariableName;
        // _finder is first assigned outside block, then reassigned inside.
        // Should only report once (at first assignment), not twice.
        let source = b"def foo\n  _finder = Model.all\n  items.each do |col|\n    _finder = _finder.where(col => val)\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense (at first assignment only), got: {:?}",
            diags
        );
    }

    #[test]
    fn test_block_local_same_name_with_later_outer_write() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"def foo(flag)\n  if flag\n    [1].each do\n      _foo = 2\n      puts _foo\n    end\n  else\n    _foo = 1\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for the block-local _foo, got: {:?}",
            diags
        );
        assert_eq!(
            diags[0].location.line, 4,
            "Expected offense on the block write"
        );
    }

    #[test]
    fn test_var_in_nested_block() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"def test_data\n  assert_raise(Error) do\n    _data = data.dup\n    _data[_data.size - 4] = 'X'\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _data, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_param_default_value_read() {
        let cop = UnderscorePrefixedVariableName;
        let source =
            b"def exists?(key, _locale = nil, locale: _locale)\n  locale || config.locale\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _locale, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_let_block_var_used() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"describe 'test' do\n  let(:record) do\n    _p = Record.last\n    _p.name = 'test'\n    _p.save\n    _p\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _p in let block, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_times_block_var_used() {
        let cop = UnderscorePrefixedVariableName;
        let source =
            b"3.times do |i|\n  _user = User.first\n  _user.name = 'test'\n  _user.save!\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _user in times block, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_included_block_var_used() {
        let cop = UnderscorePrefixedVariableName;
        // Module included block pattern (discourse)
        let source = b"module HasSearchData\n  included do\n    _name = self.name.sub('SearchData', '').underscore\n    self.primary_key = _name\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _name in included block, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_var_only_assigned_in_block_no_offense() {
        // FP test: variable assigned in a block but never read
        let cop = UnderscorePrefixedVariableName;
        let source = b"describe 'test' do\n  it 'does something' do\n    _unused = create(:record)\n    expect(1).to eq(1)\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses for unused _unused, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_multiple_blocks_same_var_name() {
        // Each block should be flagged independently (RuboCop treats each block as a scope)
        let cop = UnderscorePrefixedVariableName;
        let source = b"def test_data\n  assert_raise(Error) do\n    _data = data.dup\n    _data[_data.size - 4] = 'X'\n  end\n\n  assert_raise(Error) do\n    _data = data.dup\n    _data[_data.size - 5] = 'X'\n  end\n\n  assert_raise(Error) do\n    _data = data.dup\n    _data = _data.slice!(0, _data.size - 1)\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            3,
            "Expected 3 offenses (one per block), got: {:?}",
            diags
        );
    }

    #[test]
    fn test_different_blocks_same_var_name_no_cross_leak() {
        // FP test: two different it blocks with same variable name, only one reads it
        let cop = UnderscorePrefixedVariableName;
        let source = b"describe 'test' do\n  it 'first' do\n    _x = create(:record)\n    expect(1).to eq(1)\n  end\n\n  it 'second' do\n    _x = create(:record)\n    puts _x\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        // Only the second block should have an offense
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense (only in second block), got: {:?}",
            diags
        );
    }

    #[test]
    fn test_rescue_exception_capture_used() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"def foo\n  begin\n    risky\n  rescue StandardError => _e\n    puts _e.message\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _e rescue capture, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_rescue_exception_capture_unused() {
        let cop = UnderscorePrefixedVariableName;
        let source =
            b"def foo\n  begin\n    risky\n  rescue StandardError => _e\n    puts \"error\"\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses for unused _e, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_pattern_match_guard_bare_underscore() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"def foo(v)\n  case v\n  in _ if _.blank?\n    42\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _ in pattern guard, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_pattern_match_named_var_used() {
        let cop = UnderscorePrefixedVariableName;
        let source =
            b"def foo(parts)\n  case parts\n  in _, _, _year\n    puts _year\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _year in pattern match, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_pattern_match_var_unused() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"def foo(v)\n  case v\n  in _x\n    \"matched\"\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses for unused _x, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_destructured_block_param() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"children.each { |(_page, _children)| add(_page, _children) }\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert!(
            diags.len() >= 1,
            "Expected at least 1 offense for destructured params, got: {:?}",
            diags
        );
    }
}
