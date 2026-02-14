pub mod end_of_line;
pub mod initial_indentation;
pub mod leading_empty_lines;
pub mod line_length;
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
}
