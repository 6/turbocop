foo.map(&:to_s)
bar.select(&:valid?)
items.reject(&:nil?)
foo.map { |x| x.to_s(16) }
bar.each { |x| puts x }
baz.map { |x, y| x + y }
# Safe navigation can't be converted to &:method
items.map { |x| x&.name }
args.filter_map { |a| a&.value }
# Hash literal receiver with select/reject is unsafe (rubocop#10864)
{foo: 42}.select { |item| item.bar }
{foo: 42}.reject { |item| item.bar }
# Array literal receiver with min/max is unsafe
[1, 2, 3].min { |item| item.foo }
[1, 2, 3].max { |item| item.foo }
# Destructuring block argument (trailing comma)
something { |x,| x.first }
# Block with no arguments
something { x.method }
# Empty block body
something { |x| }
# Block with more than 1 expression
something { |x| x.method; something_else }
# Method in body not called on block arg
something { |x| y.method }
# Block with splat params
something { |*x| x.first }
# Block argument
something { |&x| x.call }
