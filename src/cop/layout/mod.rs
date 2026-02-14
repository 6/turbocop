pub mod empty_lines;
pub mod end_of_line;
pub mod initial_indentation;
pub mod leading_empty_lines;
pub mod line_length;
pub mod space_after_colon;
pub mod space_after_comma;
pub mod space_after_semicolon;
pub mod space_around_equals_in_parameter_default;
pub mod space_before_block_braces;
pub mod space_before_comma;
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
}
