pub mod argument_alignment;
pub mod array_alignment;
pub mod assignment_indentation;
pub mod block_alignment;
pub mod case_indentation;
pub mod comment_indentation;
pub mod condition_position;
pub mod def_end_alignment;
pub mod else_alignment;
pub mod empty_line_after_magic_comment;
pub mod empty_line_between_defs;
pub mod empty_lines;
pub mod empty_lines_around_access_modifier;
pub mod empty_lines_around_block_body;
pub mod empty_lines_around_class_body;
pub mod empty_lines_around_method_body;
pub mod empty_lines_around_module_body;
pub mod end_alignment;
pub mod end_of_line;
pub mod first_argument_indentation;
pub mod first_array_element_indentation;
pub mod first_hash_element_indentation;
pub mod hash_alignment;
pub mod indentation_consistency;
pub mod indentation_width;
pub mod initial_indentation;
pub mod leading_comment_space;
pub mod leading_empty_lines;
pub mod line_length;
pub mod multiline_method_call_indentation;
pub mod multiline_operation_indentation;
pub mod rescue_ensure_alignment;
pub mod space_after_colon;
pub mod space_after_comma;
pub mod space_after_semicolon;
pub mod space_around_equals_in_parameter_default;
pub mod space_around_keyword;
pub mod space_around_operators;
pub mod space_before_block_braces;
pub mod space_before_comma;
pub mod space_before_comment;
pub mod space_before_first_arg;
pub mod space_inside_array_literal_brackets;
pub mod space_inside_block_braces;
pub mod space_inside_hash_literal_braces;
pub mod space_inside_parens;
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
}
