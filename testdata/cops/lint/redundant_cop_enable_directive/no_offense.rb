# rubocop:disable Layout/LineLength
foooooooooooooooooooooooooooo = 1
# rubocop:enable Layout/LineLength
bar
# rubocop:disable Layout
x =   0
# rubocop:enable Layout
y = 2
# rubocop:disable Layout/LineLength
foooooo = 1
# rubocop:enable all

# Directives inside heredocs should not be detected
code = <<~RUBY
  foo = 1
  # rubocop:enable Layout/LineLength
  bar = 2
RUBY
puts code
