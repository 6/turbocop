[1, 2, 3].include?(e)
{ foo: :bar }.key?(:foo)
loop do
  [1, nil, 3].compact!
end
loop do
  { foo: :bar, baz: nil }.select! { |_k, v| !v.nil? }
end
loop do
  array = [1, nil, 3]
end
loop do
  hash = { foo: :bar, baz: nil }
end
[1, 2, variable].include?(e)
{ foo: { bar: variable } }.key?(:foo)
[[1, 2, 3] | [2, 3, 4]].each { |e| puts e }
loop do
  array.all? { |x| x > 100 }
end
while x < 100
  puts x
end
# Safe navigation (&.) should NOT be treated as a loop
items&.each { |item| [1, 2, 3].include?(item) }
items&.map { |item| [1, 2, 3].include?(item) }
items&.select { |item| { a: 1 }.key?(item) }
# Interpolated regex is NOT a basic literal — should not be flagged
items.each { |item| [/foo/, /#{item}/].any? { |r| "str".match?(r) } }
# Nested loops with same literal as receiver: not flagged (RuboCop value equality)
[1].each { |x| [1].each { puts x } }
[true, false].each do |a|
  [true, false].each do |b|
    puts a, b
  end
end
%i[aws azure gcp].each do |src|
  %i[aws azure gcp].each do |dst|
    puts src, dst
  end
end
# Literal used with - operator where literal equals receiver of enclosing loop
%i[subscribe assign].each do |mode_name|
  other_modes = %i[subscribe assign] - [mode_name]
end
# Literal as receiver of loop inside another loop with different receiver (array arg of -)
([:install, "install", :gem, "gem"] - [type]).each do |other|
  puts other
end
# Literal that is a descendant of the loop receiver should not be flagged
[1, 2].sort.each { |x| puts x }
# Literal in arguments to zip without a block (not a loop)
[1, 2].zip([3, 4])
