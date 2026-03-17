def foo
  puts "hello"
end

# Regular comment
x = 1
y = 2
z = x + y

# Directives inside string literals should not be flagged
code = <<~RUBY
  # rubocop:disable Metrics/MethodLength
  def long_method
  end
  # rubocop:enable Metrics/MethodLength
RUBY

template = "# rubocop:disable Style/FrozenStringLiteralComment"

multiline = <<-HEREDOC
  # rubocop:todo Layout/LineLength
  some_long_line
HEREDOC

# YARD doc comments that mention directives in prose should not trigger
# Checks that `# rubocop:enable ...` and `# rubocop:disable ...` statements
# are balanced within a source file.

# Example comment referencing a directive:
#   # rubocop:disable all
# The above is just documentation, not an actual directive.
