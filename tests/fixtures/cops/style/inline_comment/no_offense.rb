# A standalone comment
two = 1 + 1
# Another standalone comment
x = 42
two = 1 + 1 # rubocop:disable Layout/ExtraSpacing

# Hash inside regex is not a comment
PATTERN = /\A---(\s+#|\s*\z)/.freeze
x = /foo#bar/
y = %r{path#fragment}

# Hash inside string interpolation is not a comment
result = "hello #{world}"
msg = "count: #{items.size}"

# Hash inside percent literals is not a comment
z = %q{hello # world}
w = %Q{value #{name}}

# Hash inside heredoc is not a comment
text = <<~HEREDOC
  some # text
HEREDOC
