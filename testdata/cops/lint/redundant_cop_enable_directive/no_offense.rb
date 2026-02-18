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

# Doc comment containing embedded rubocop:disable counts as a real disable
# so the subsequent enable is valid (not redundant)
#   def f # rubocop:disable Style/For
#   end
# rubocop:enable Style/For

# Documentation text with embedded examples should not be treated as directives
# Checks that `# rubocop:enable ...` and `# rubocop:disable ...` statements
# are strictly formatted.

# Plain comment mentioning rubocop:enable in prose is not a directive
# removed. This is done in order to find rubocop:enable directives that
# have now become useless.
