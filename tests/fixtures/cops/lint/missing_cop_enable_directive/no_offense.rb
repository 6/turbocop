# rubocop:disable Layout/SpaceAroundOperators
x =   0
# rubocop:enable Layout/SpaceAroundOperators
# Some other code
# rubocop:disable Layout
x =   0
# rubocop:enable Layout
# Some other code
x = 1 # rubocop:disable Layout/LineLength
y = 2

# Directives inside heredocs should not be detected
code = <<~RUBY
  # rubocop:disable Layout/LineLength
  very_long_line = 1
RUBY
puts code
