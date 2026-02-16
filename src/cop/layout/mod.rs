pub mod access_modifier_indentation;
pub mod argument_alignment;
pub mod array_alignment;
pub mod assignment_indentation;
pub mod begin_end_alignment;
pub mod block_alignment;
pub mod block_end_newline;
pub mod case_indentation;
pub mod class_structure;
pub mod closing_heredoc_indentation;
pub mod closing_parenthesis_indentation;
pub mod comment_indentation;
pub mod condition_position;
pub mod def_end_alignment;
pub mod dot_position;
pub mod else_alignment;
pub mod empty_comment;
pub mod empty_line_after_guard_clause;
pub mod empty_line_after_magic_comment;
pub mod empty_line_after_multiline_condition;
pub mod empty_line_between_defs;
pub mod empty_lines;
pub mod empty_lines_after_module_inclusion;
pub mod empty_lines_around_access_modifier;
pub mod empty_lines_around_arguments;
pub mod empty_lines_around_attribute_accessor;
pub mod empty_lines_around_begin_body;
pub mod empty_lines_around_block_body;
pub mod empty_lines_around_class_body;
pub mod empty_lines_around_exception_handling_keywords;
pub mod empty_lines_around_method_body;
pub mod empty_lines_around_module_body;
pub mod end_alignment;
pub mod end_of_line;
pub mod extra_spacing;
pub mod first_argument_indentation;
pub mod first_array_element_indentation;
pub mod first_array_element_line_break;
pub mod first_hash_element_indentation;
pub mod first_hash_element_line_break;
pub mod first_method_argument_line_break;
pub mod first_method_parameter_line_break;
pub mod first_parameter_indentation;
pub mod hash_alignment;
pub mod heredoc_argument_closing_parenthesis;
pub mod heredoc_indentation;
pub mod indentation_consistency;
pub mod indentation_style;
pub mod indentation_width;
pub mod initial_indentation;
pub mod leading_comment_space;
pub mod leading_empty_lines;
pub mod line_continuation_leading_space;
pub mod line_continuation_spacing;
pub mod line_end_string_concatenation_indentation;
pub mod line_length;
pub mod multiline_array_brace_layout;
pub mod multiline_array_line_breaks;
pub mod multiline_assignment_layout;
pub mod multiline_block_layout;
pub mod multiline_hash_brace_layout;
pub mod multiline_hash_key_line_breaks;
pub mod multiline_method_argument_line_breaks;
pub mod multiline_method_call_brace_layout;
pub mod multiline_method_call_indentation;
pub mod multiline_method_definition_brace_layout;
pub mod multiline_method_parameter_line_breaks;
pub mod multiline_operation_indentation;
pub mod parameter_alignment;
pub mod redundant_line_break;
pub mod rescue_ensure_alignment;
pub mod single_line_block_chain;
pub mod space_after_colon;
pub mod space_after_comma;
pub mod space_after_method_name;
pub mod space_after_not;
pub mod space_after_semicolon;
pub mod space_around_block_parameters;
pub mod space_around_equals_in_parameter_default;
pub mod space_around_keyword;
pub mod space_around_method_call_operator;
pub mod space_around_operators;
pub mod space_before_block_braces;
pub mod space_before_brackets;
pub mod space_before_comma;
pub mod space_before_comment;
pub mod space_before_first_arg;
pub mod space_before_semicolon;
pub mod space_in_lambda_literal;
pub mod space_inside_array_literal_brackets;
pub mod space_inside_array_percent_literal;
pub mod space_inside_block_braces;
pub mod space_inside_hash_literal_braces;
pub mod space_inside_parens;
pub mod space_inside_percent_literal_delimiters;
pub mod space_inside_range_literal;
pub mod space_inside_reference_brackets;
pub mod space_inside_string_interpolation;
pub mod trailing_empty_lines;
pub mod trailing_whitespace;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(trailing_whitespace::TrailingWhitespace));
    registry.register(Box::new(line_length::LineLength));
    registry.register(Box::new(trailing_empty_lines::TrailingEmptyLines));
    registry.register(Box::new(leading_empty_lines::LeadingEmptyLines));
    registry.register(Box::new(end_of_line::EndOfLine));
    registry.register(Box::new(initial_indentation::InitialIndentation));
    registry.register(Box::new(empty_lines::EmptyLines));
    registry.register(Box::new(space_after_comma::SpaceAfterComma));
    registry.register(Box::new(space_after_semicolon::SpaceAfterSemicolon));
    registry.register(Box::new(space_before_comma::SpaceBeforeComma));
    registry.register(Box::new(
        space_around_equals_in_parameter_default::SpaceAroundEqualsInParameterDefault,
    ));
    registry.register(Box::new(space_after_colon::SpaceAfterColon));
    registry.register(Box::new(space_inside_parens::SpaceInsideParens));
    registry.register(Box::new(
        space_inside_hash_literal_braces::SpaceInsideHashLiteralBraces,
    ));
    registry.register(Box::new(space_inside_block_braces::SpaceInsideBlockBraces));
    registry.register(Box::new(
        space_inside_array_literal_brackets::SpaceInsideArrayLiteralBrackets,
    ));
    registry.register(Box::new(space_before_block_braces::SpaceBeforeBlockBraces));
    // M5 cops
    registry.register(Box::new(
        empty_line_between_defs::EmptyLineBetweenDefs,
    ));
    registry.register(Box::new(
        empty_lines_around_class_body::EmptyLinesAroundClassBody,
    ));
    registry.register(Box::new(
        empty_lines_around_module_body::EmptyLinesAroundModuleBody,
    ));
    registry.register(Box::new(
        empty_lines_around_method_body::EmptyLinesAroundMethodBody,
    ));
    registry.register(Box::new(
        empty_lines_around_block_body::EmptyLinesAroundBlockBody,
    ));
    registry.register(Box::new(case_indentation::CaseIndentation));
    registry.register(Box::new(argument_alignment::ArgumentAlignment));
    registry.register(Box::new(array_alignment::ArrayAlignment));
    registry.register(Box::new(hash_alignment::HashAlignment));
    registry.register(Box::new(block_alignment::BlockAlignment));
    registry.register(Box::new(condition_position::ConditionPosition));
    registry.register(Box::new(def_end_alignment::DefEndAlignment));
    registry.register(Box::new(else_alignment::ElseAlignment));
    registry.register(Box::new(end_alignment::EndAlignment));
    registry.register(Box::new(
        rescue_ensure_alignment::RescueEnsureAlignment,
    ));
    registry.register(Box::new(indentation_width::IndentationWidth));
    registry.register(Box::new(
        indentation_consistency::IndentationConsistency,
    ));
    registry.register(Box::new(
        first_argument_indentation::FirstArgumentIndentation,
    ));
    registry.register(Box::new(
        first_array_element_indentation::FirstArrayElementIndentation,
    ));
    registry.register(Box::new(
        first_hash_element_indentation::FirstHashElementIndentation,
    ));
    registry.register(Box::new(
        assignment_indentation::AssignmentIndentation,
    ));
    registry.register(Box::new(
        multiline_method_call_indentation::MultilineMethodCallIndentation,
    ));
    registry.register(Box::new(
        multiline_operation_indentation::MultilineOperationIndentation,
    ));
    registry.register(Box::new(
        space_around_operators::SpaceAroundOperators,
    ));
    registry.register(Box::new(
        space_around_keyword::SpaceAroundKeyword,
    ));
    registry.register(Box::new(
        space_before_comment::SpaceBeforeComment,
    ));
    registry.register(Box::new(
        space_before_first_arg::SpaceBeforeFirstArg,
    ));
    registry.register(Box::new(
        leading_comment_space::LeadingCommentSpace,
    ));
    registry.register(Box::new(
        comment_indentation::CommentIndentation,
    ));
    registry.register(Box::new(
        empty_line_after_magic_comment::EmptyLineAfterMagicComment,
    ));
    registry.register(Box::new(
        empty_lines_around_access_modifier::EmptyLinesAroundAccessModifier,
    ));
    // New cops
    registry.register(Box::new(
        access_modifier_indentation::AccessModifierIndentation,
    ));
    registry.register(Box::new(block_end_newline::BlockEndNewline));
    registry.register(Box::new(
        closing_parenthesis_indentation::ClosingParenthesisIndentation,
    ));
    registry.register(Box::new(dot_position::DotPosition));
    registry.register(Box::new(empty_comment::EmptyComment));
    registry.register(Box::new(
        empty_line_after_guard_clause::EmptyLineAfterGuardClause,
    ));
    registry.register(Box::new(
        empty_lines_around_begin_body::EmptyLinesAroundBeginBody,
    ));
    registry.register(Box::new(
        empty_lines_around_exception_handling_keywords::EmptyLinesAroundExceptionHandlingKeywords,
    ));
    registry.register(Box::new(indentation_style::IndentationStyle));
    registry.register(Box::new(multiline_block_layout::MultilineBlockLayout));
    registry.register(Box::new(parameter_alignment::ParameterAlignment));
    registry.register(Box::new(space_after_method_name::SpaceAfterMethodName));
    registry.register(Box::new(space_after_not::SpaceAfterNot));
    registry.register(Box::new(space_before_semicolon::SpaceBeforeSemicolon));
    registry.register(Box::new(space_in_lambda_literal::SpaceInLambdaLiteral));
    registry.register(Box::new(
        space_inside_range_literal::SpaceInsideRangeLiteral,
    ));
    registry.register(Box::new(
        space_inside_string_interpolation::SpaceInsideStringInterpolation,
    ));
    registry.register(Box::new(
        space_inside_percent_literal_delimiters::SpaceInsidePercentLiteralDelimiters,
    ));
    registry.register(Box::new(
        space_inside_array_percent_literal::SpaceInsideArrayPercentLiteral,
    ));
    registry.register(Box::new(space_before_brackets::SpaceBeforeBrackets));
    // New layout cops
    registry.register(Box::new(
        closing_heredoc_indentation::ClosingHeredocIndentation,
    ));
    registry.register(Box::new(heredoc_indentation::HeredocIndentation));
    registry.register(Box::new(extra_spacing::ExtraSpacing));
    registry.register(Box::new(
        space_around_method_call_operator::SpaceAroundMethodCallOperator,
    ));
    registry.register(Box::new(
        space_inside_reference_brackets::SpaceInsideReferenceBrackets,
    ));
    registry.register(Box::new(
        empty_lines_around_attribute_accessor::EmptyLinesAroundAttributeAccessor,
    ));
    registry.register(Box::new(
        empty_lines_after_module_inclusion::EmptyLinesAfterModuleInclusion,
    ));
    registry.register(Box::new(
        line_continuation_leading_space::LineContinuationLeadingSpace,
    ));
    registry.register(Box::new(
        multiline_array_brace_layout::MultilineArrayBraceLayout,
    ));
    registry.register(Box::new(
        multiline_hash_brace_layout::MultilineHashBraceLayout,
    ));
    // Batch: 17 existing cops
    registry.register(Box::new(
        begin_end_alignment::BeginEndAlignment,
    ));
    registry.register(Box::new(class_structure::ClassStructure));
    registry.register(Box::new(
        empty_line_after_multiline_condition::EmptyLineAfterMultilineCondition,
    ));
    registry.register(Box::new(
        empty_lines_around_arguments::EmptyLinesAroundArguments,
    ));
    registry.register(Box::new(
        first_array_element_line_break::FirstArrayElementLineBreak,
    ));
    registry.register(Box::new(
        first_hash_element_line_break::FirstHashElementLineBreak,
    ));
    registry.register(Box::new(
        first_method_argument_line_break::FirstMethodArgumentLineBreak,
    ));
    registry.register(Box::new(
        first_method_parameter_line_break::FirstMethodParameterLineBreak,
    ));
    registry.register(Box::new(
        first_parameter_indentation::FirstParameterIndentation,
    ));
    registry.register(Box::new(
        heredoc_argument_closing_parenthesis::HeredocArgumentClosingParenthesis,
    ));
    registry.register(Box::new(
        line_continuation_spacing::LineContinuationSpacing,
    ));
    registry.register(Box::new(
        line_end_string_concatenation_indentation::LineEndStringConcatenationIndentation,
    ));
    registry.register(Box::new(
        multiline_array_line_breaks::MultilineArrayLineBreaks,
    ));
    registry.register(Box::new(
        multiline_hash_key_line_breaks::MultilineHashKeyLineBreaks,
    ));
    registry.register(Box::new(
        multiline_method_argument_line_breaks::MultilineMethodArgumentLineBreaks,
    ));
    registry.register(Box::new(
        multiline_method_parameter_line_breaks::MultilineMethodParameterLineBreaks,
    ));
    registry.register(Box::new(
        space_around_block_parameters::SpaceAroundBlockParameters,
    ));
    // Batch: 5 new cops
    registry.register(Box::new(
        multiline_assignment_layout::MultilineAssignmentLayout,
    ));
    registry.register(Box::new(
        multiline_method_call_brace_layout::MultilineMethodCallBraceLayout,
    ));
    registry.register(Box::new(
        multiline_method_definition_brace_layout::MultilineMethodDefinitionBraceLayout,
    ));
    registry.register(Box::new(
        redundant_line_break::RedundantLineBreak,
    ));
    registry.register(Box::new(
        single_line_block_chain::SingleLineBlockChain,
    ));
}
