pub mod ambiguous_assignment;
pub mod ambiguous_block_association;
pub mod ambiguous_operator;
pub mod ambiguous_operator_precedence;
pub mod ambiguous_range;
pub mod ambiguous_regexp_literal;
pub mod array_literal_in_regexp;
pub mod assignment_in_condition;
pub mod big_decimal_new;
pub mod binary_operator_with_identical_operands;
pub mod boolean_symbol;
pub mod circular_argument_reference;
pub mod constant_definition_in_block;
pub mod constant_overwritten_in_rescue;
pub mod constant_reassignment;
pub mod constant_resolution;
pub mod cop_directive_syntax;
pub mod debugger;
pub mod deprecated_class_methods;
pub mod deprecated_constants;
pub mod deprecated_open_ssl_constant;
pub mod disjunctive_assignment_in_constructor;
pub mod duplicate_branch;
pub mod duplicate_case_condition;
pub mod duplicate_elsif_condition;
pub mod duplicate_hash_key;
pub mod duplicate_magic_comment;
pub mod duplicate_match_pattern;
pub mod duplicate_methods;
pub mod duplicate_regexp_character_class_element;
pub mod duplicate_require;
pub mod duplicate_rescue_exception;
pub mod duplicate_set_element;
pub mod each_with_object_argument;
pub mod else_layout;
pub mod empty_block;
pub mod empty_class;
pub mod empty_conditional_body;
pub mod empty_ensure;
pub mod empty_expression;
pub mod empty_file;
pub mod empty_in_pattern;
pub mod empty_interpolation;
pub mod empty_when;
pub mod ensure_return;
pub mod erb_new_arguments;
pub mod flip_flop;
pub mod float_comparison;
pub mod float_out_of_range;
pub mod format_parameter_mismatch;
pub mod hash_compare_by_identity;
pub mod hash_new_with_keyword_arguments_as_default;
pub mod heredoc_method_call_position;
pub mod identity_comparison;
pub mod implicit_string_concatenation;
pub mod incompatible_io_select_with_fiber_scheduler;
pub mod ineffective_access_modifier;
pub mod inherit_exception;
pub mod interpolation_check;
pub mod it_without_arguments_in_block;
pub mod lambda_without_literal_block;
pub mod literal_as_condition;
pub mod literal_assignment_in_condition;
pub mod literal_in_interpolation;
pub mod loop_cop;
pub mod missing_cop_enable_directive;
pub mod missing_super;
pub mod mixed_case_range;
pub mod mixed_regexp_capture_types;
pub mod multiple_comparison;
pub mod nested_method_definition;
pub mod nested_percent_literal;
pub mod next_without_accumulator;
pub mod no_return_in_begin_end_blocks;
pub mod non_atomic_file_operation;
pub mod non_deterministic_require_order;
pub mod non_local_exit_from_iterator;
pub mod number_conversion;
pub mod numbered_parameter_assignment;
pub mod numeric_operation_with_constant_result;
pub mod or_assignment_to_constant;
pub mod ordered_magic_comments;
pub mod out_of_range_regexp_ref;
pub mod parentheses_as_grouped_expression;
pub mod percent_string_array;
pub mod percent_symbol_array;
pub mod raise_exception;
pub mod rand_one;
pub mod redundant_cop_disable_directive;
pub mod redundant_cop_enable_directive;
pub mod redundant_dir_glob_sort;
pub mod redundant_regexp_quantifiers;
pub mod redundant_require_statement;
pub mod redundant_safe_navigation;
pub mod redundant_splat_expansion;
pub mod redundant_string_coercion;
pub mod redundant_type_conversion;
pub mod redundant_with_index;
pub mod redundant_with_object;
pub mod refinement_import_methods;
pub mod regexp_as_condition;
pub mod require_parentheses;
pub mod require_range_parentheses;
pub mod require_relative_self_path;
pub mod rescue_exception;
pub mod rescue_type;
pub mod return_in_void_context;
pub mod safe_navigation_chain;
pub mod safe_navigation_consistency;
pub mod safe_navigation_with_empty;
pub mod script_permission;
pub mod self_assignment;
pub mod send_with_mixin_argument;
pub mod shadowed_argument;
pub mod shadowed_exception;
pub mod shadowing_outer_local_variable;
pub mod shared_mutable_default;
pub mod struct_new_override;
pub mod suppressed_exception;
pub mod suppressed_exception_in_number_conversion;
pub mod symbol_conversion;
pub mod syntax;
pub mod to_enum_arguments;
pub mod to_json;
pub mod top_level_return_with_argument;
pub mod trailing_comma_in_attribute_declaration;
pub mod triple_quotes;
pub mod underscore_prefixed_variable_name;
pub mod unescaped_bracket_in_regexp;
pub mod unexpected_block_arity;
pub mod unified_integer;
pub mod unmodified_reduce_accumulator;
pub mod unreachable_code;
pub mod unreachable_loop;
pub mod unused_block_argument;
pub mod unused_method_argument;
pub mod uri_escape_unescape;
pub mod uri_regexp;
pub mod useless_access_modifier;
pub mod useless_assignment;
pub mod useless_constant_scoping;
pub mod useless_default_value_argument;
pub mod useless_defined;
pub mod useless_else_without_rescue;
pub mod useless_method_definition;
pub mod useless_numeric_operation;
pub mod useless_or;
pub mod useless_rescue;
pub mod useless_ruby2_keywords;
pub mod useless_setter_call;
pub mod useless_times;
pub mod void;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(ambiguous_assignment::AmbiguousAssignment));
    registry.register(Box::new(
        ambiguous_block_association::AmbiguousBlockAssociation,
    ));
    registry.register(Box::new(ambiguous_operator::AmbiguousOperator));
    registry.register(Box::new(
        ambiguous_operator_precedence::AmbiguousOperatorPrecedence,
    ));
    registry.register(Box::new(ambiguous_range::AmbiguousRange));
    registry.register(Box::new(ambiguous_regexp_literal::AmbiguousRegexpLiteral));
    registry.register(Box::new(array_literal_in_regexp::ArrayLiteralInRegexp));
    registry.register(Box::new(assignment_in_condition::AssignmentInCondition));
    registry.register(Box::new(big_decimal_new::BigDecimalNew));
    registry.register(Box::new(
        binary_operator_with_identical_operands::BinaryOperatorWithIdenticalOperands,
    ));
    registry.register(Box::new(boolean_symbol::BooleanSymbol));
    registry.register(Box::new(
        circular_argument_reference::CircularArgumentReference,
    ));
    registry.register(Box::new(cop_directive_syntax::CopDirectiveSyntax));
    registry.register(Box::new(
        constant_definition_in_block::ConstantDefinitionInBlock,
    ));
    registry.register(Box::new(
        constant_overwritten_in_rescue::ConstantOverwrittenInRescue,
    ));
    registry.register(Box::new(constant_reassignment::ConstantReassignment));
    registry.register(Box::new(constant_resolution::ConstantResolution));
    registry.register(Box::new(debugger::Debugger));
    registry.register(Box::new(deprecated_class_methods::DeprecatedClassMethods));
    registry.register(Box::new(
        deprecated_open_ssl_constant::DeprecatedOpenSSLConstant,
    ));
    registry.register(Box::new(deprecated_constants::DeprecatedConstants));
    registry.register(Box::new(
        disjunctive_assignment_in_constructor::DisjunctiveAssignmentInConstructor,
    ));
    registry.register(Box::new(duplicate_branch::DuplicateBranch));
    registry.register(Box::new(duplicate_case_condition::DuplicateCaseCondition));
    registry.register(Box::new(duplicate_elsif_condition::DuplicateElsifCondition));
    registry.register(Box::new(duplicate_hash_key::DuplicateHashKey));
    registry.register(Box::new(duplicate_magic_comment::DuplicateMagicComment));
    registry.register(Box::new(duplicate_match_pattern::DuplicateMatchPattern));
    registry.register(Box::new(duplicate_methods::DuplicateMethods));
    registry.register(Box::new(
        duplicate_regexp_character_class_element::DuplicateRegexpCharacterClassElement,
    ));
    registry.register(Box::new(duplicate_require::DuplicateRequire));
    registry.register(Box::new(
        duplicate_rescue_exception::DuplicateRescueException,
    ));
    registry.register(Box::new(duplicate_set_element::DuplicateSetElement));
    registry.register(Box::new(each_with_object_argument::EachWithObjectArgument));
    registry.register(Box::new(else_layout::ElseLayout));
    registry.register(Box::new(empty_block::EmptyBlock));
    registry.register(Box::new(empty_class::EmptyClass));
    registry.register(Box::new(empty_conditional_body::EmptyConditionalBody));
    registry.register(Box::new(empty_ensure::EmptyEnsure));
    registry.register(Box::new(empty_expression::EmptyExpression));
    registry.register(Box::new(empty_in_pattern::EmptyInPattern));
    registry.register(Box::new(empty_file::EmptyFile));
    registry.register(Box::new(empty_interpolation::EmptyInterpolation));
    registry.register(Box::new(empty_when::EmptyWhen));
    registry.register(Box::new(ensure_return::EnsureReturn));
    registry.register(Box::new(erb_new_arguments::ErbNewArguments));
    registry.register(Box::new(flip_flop::FlipFlop));
    registry.register(Box::new(hash_compare_by_identity::HashCompareByIdentity));
    registry.register(Box::new(
        hash_new_with_keyword_arguments_as_default::HashNewWithKeywordArgumentsAsDefault,
    ));
    registry.register(Box::new(float_comparison::FloatComparison));
    registry.register(Box::new(float_out_of_range::FloatOutOfRange));
    registry.register(Box::new(format_parameter_mismatch::FormatParameterMismatch));
    registry.register(Box::new(
        heredoc_method_call_position::HeredocMethodCallPosition,
    ));
    registry.register(Box::new(identity_comparison::IdentityComparison));
    registry.register(Box::new(
        implicit_string_concatenation::ImplicitStringConcatenation,
    ));
    registry.register(Box::new(
        incompatible_io_select_with_fiber_scheduler::IncompatibleIoSelectWithFiberScheduler,
    ));
    registry.register(Box::new(
        ineffective_access_modifier::IneffectiveAccessModifier,
    ));
    registry.register(Box::new(inherit_exception::InheritException));
    registry.register(Box::new(interpolation_check::InterpolationCheck));
    registry.register(Box::new(
        it_without_arguments_in_block::ItWithoutArgumentsInBlock,
    ));
    registry.register(Box::new(
        lambda_without_literal_block::LambdaWithoutLiteralBlock,
    ));
    registry.register(Box::new(literal_as_condition::LiteralAsCondition));
    registry.register(Box::new(
        literal_assignment_in_condition::LiteralAssignmentInCondition,
    ));
    registry.register(Box::new(literal_in_interpolation::LiteralInInterpolation));
    registry.register(Box::new(loop_cop::Loop));
    registry.register(Box::new(missing_super::MissingSuper));
    registry.register(Box::new(mixed_case_range::MixedCaseRange));
    registry.register(Box::new(
        mixed_regexp_capture_types::MixedRegexpCaptureTypes,
    ));
    registry.register(Box::new(
        missing_cop_enable_directive::MissingCopEnableDirective,
    ));
    registry.register(Box::new(multiple_comparison::MultipleComparison));
    registry.register(Box::new(nested_method_definition::NestedMethodDefinition));
    registry.register(Box::new(next_without_accumulator::NextWithoutAccumulator));
    registry.register(Box::new(
        no_return_in_begin_end_blocks::NoReturnInBeginEndBlocks,
    ));
    registry.register(Box::new(non_atomic_file_operation::NonAtomicFileOperation));
    registry.register(Box::new(
        non_deterministic_require_order::NonDeterministicRequireOrder,
    ));
    registry.register(Box::new(number_conversion::NumberConversion));
    registry.register(Box::new(
        numbered_parameter_assignment::NumberedParameterAssignment,
    ));
    registry.register(Box::new(
        numeric_operation_with_constant_result::NumericOperationWithConstantResult,
    ));
    registry.register(Box::new(nested_percent_literal::NestedPercentLiteral));
    registry.register(Box::new(
        non_local_exit_from_iterator::NonLocalExitFromIterator,
    ));
    registry.register(Box::new(or_assignment_to_constant::OrAssignmentToConstant));
    registry.register(Box::new(ordered_magic_comments::OrderedMagicComments));
    registry.register(Box::new(out_of_range_regexp_ref::OutOfRangeRegexpRef));
    registry.register(Box::new(
        parentheses_as_grouped_expression::ParenthesesAsGroupedExpression,
    ));
    registry.register(Box::new(percent_string_array::PercentStringArray));
    registry.register(Box::new(percent_symbol_array::PercentSymbolArray));
    registry.register(Box::new(raise_exception::RaiseException));
    registry.register(Box::new(rand_one::RandOne));
    registry.register(Box::new(
        redundant_cop_disable_directive::RedundantCopDisableDirective,
    ));
    registry.register(Box::new(
        redundant_cop_enable_directive::RedundantCopEnableDirective,
    ));
    registry.register(Box::new(redundant_dir_glob_sort::RedundantDirGlobSort));
    registry.register(Box::new(
        redundant_regexp_quantifiers::RedundantRegexpQuantifiers,
    ));
    registry.register(Box::new(
        redundant_require_statement::RedundantRequireStatement,
    ));
    registry.register(Box::new(redundant_safe_navigation::RedundantSafeNavigation));
    registry.register(Box::new(redundant_splat_expansion::RedundantSplatExpansion));
    registry.register(Box::new(redundant_string_coercion::RedundantStringCoercion));
    registry.register(Box::new(redundant_type_conversion::RedundantTypeConversion));
    registry.register(Box::new(redundant_with_index::RedundantWithIndex));
    registry.register(Box::new(redundant_with_object::RedundantWithObject));
    registry.register(Box::new(refinement_import_methods::RefinementImportMethods));
    registry.register(Box::new(regexp_as_condition::RegexpAsCondition));
    registry.register(Box::new(require_parentheses::RequireParentheses));
    registry.register(Box::new(require_range_parentheses::RequireRangeParentheses));
    registry.register(Box::new(
        require_relative_self_path::RequireRelativeSelfPath,
    ));
    registry.register(Box::new(rescue_exception::RescueException));
    registry.register(Box::new(rescue_type::RescueType));
    registry.register(Box::new(return_in_void_context::ReturnInVoidContext));
    registry.register(Box::new(safe_navigation_chain::SafeNavigationChain));
    registry.register(Box::new(
        safe_navigation_consistency::SafeNavigationConsistency,
    ));
    registry.register(Box::new(
        safe_navigation_with_empty::SafeNavigationWithEmpty,
    ));
    registry.register(Box::new(script_permission::ScriptPermission));
    registry.register(Box::new(self_assignment::SelfAssignment));
    registry.register(Box::new(send_with_mixin_argument::SendWithMixinArgument));
    registry.register(Box::new(shadowed_argument::ShadowedArgument));
    registry.register(Box::new(shadowed_exception::ShadowedException));
    registry.register(Box::new(
        shadowing_outer_local_variable::ShadowingOuterLocalVariable,
    ));
    registry.register(Box::new(shared_mutable_default::SharedMutableDefault));
    registry.register(Box::new(struct_new_override::StructNewOverride));
    registry.register(Box::new(suppressed_exception::SuppressedException));
    registry.register(Box::new(
        suppressed_exception_in_number_conversion::SuppressedExceptionInNumberConversion,
    ));
    registry.register(Box::new(symbol_conversion::SymbolConversion));
    registry.register(Box::new(syntax::Syntax));
    registry.register(Box::new(to_enum_arguments::ToEnumArguments));
    registry.register(Box::new(to_json::ToJSON));
    registry.register(Box::new(
        top_level_return_with_argument::TopLevelReturnWithArgument,
    ));
    registry.register(Box::new(
        trailing_comma_in_attribute_declaration::TrailingCommaInAttributeDeclaration,
    ));
    registry.register(Box::new(triple_quotes::TripleQuotes));
    registry.register(Box::new(
        underscore_prefixed_variable_name::UnderscorePrefixedVariableName,
    ));
    registry.register(Box::new(
        unescaped_bracket_in_regexp::UnescapedBracketInRegexp,
    ));
    registry.register(Box::new(unexpected_block_arity::UnexpectedBlockArity));
    registry.register(Box::new(unified_integer::UnifiedInteger));
    registry.register(Box::new(
        unmodified_reduce_accumulator::UnmodifiedReduceAccumulator,
    ));
    registry.register(Box::new(unreachable_code::UnreachableCode));
    registry.register(Box::new(unreachable_loop::UnreachableLoop));
    registry.register(Box::new(unused_block_argument::UnusedBlockArgument));
    registry.register(Box::new(unused_method_argument::UnusedMethodArgument));
    registry.register(Box::new(uri_escape_unescape::UriEscapeUnescape));
    registry.register(Box::new(uri_regexp::UriRegexp));
    registry.register(Box::new(useless_access_modifier::UselessAccessModifier));
    registry.register(Box::new(useless_assignment::UselessAssignment));
    registry.register(Box::new(useless_constant_scoping::UselessConstantScoping));
    registry.register(Box::new(
        useless_default_value_argument::UselessDefaultValueArgument,
    ));
    registry.register(Box::new(useless_defined::UselessDefined));
    registry.register(Box::new(
        useless_else_without_rescue::UselessElseWithoutRescue,
    ));
    registry.register(Box::new(useless_method_definition::UselessMethodDefinition));
    registry.register(Box::new(useless_numeric_operation::UselessNumericOperation));
    registry.register(Box::new(useless_or::UselessOr));
    registry.register(Box::new(useless_rescue::UselessRescue));
    registry.register(Box::new(useless_ruby2_keywords::UselessRuby2Keywords));
    registry.register(Box::new(useless_setter_call::UselessSetterCall));
    registry.register(Box::new(useless_times::UselessTimes));
    registry.register(Box::new(void::Void));
}
