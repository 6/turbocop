while true
  [1, 2, 3].include?(e)
  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.
end
while i < 100
  { foo: :bar }.key?(:foo)
  ^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Hash literals in loops. It is better to extract it into a local variable or a constant.
end
until i < 100
  [1, 2, 3].include?(e)
  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.
end
for i in 1..100
  { foo: :bar }.key?(:foo)
  ^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Hash literals in loops. It is better to extract it into a local variable or a constant.
end
loop do
  [1, 2, 3].include?(e)
  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.
end
array.all? do |e|
  [1, 2, 3].include?(e)
  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.
end
array.each do |e|
  { foo: :bar }.key?(:foo)
  ^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Hash literals in loops. It is better to extract it into a local variable or a constant.
end
items.each do |item|
  %w[foo bar baz].each do |type|
  ^^^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.
    do_something(item, type)
  end
end
items.each do |item|
  [true, false].each do |flag|
  ^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.
    do_something(item, flag)
  end
end
loop do
  [1, 2, 3].each do |n|
  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.
    puts n
  end
end
items.each do |item|
  { a: 1, b: 2 }.each do |k, v|
  ^^^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Hash literals in loops. It is better to extract it into a local variable or a constant.
    do_something(item, k, v)
  end
end
while true
  %w[x y z].map do |s|
  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.
    s.upcase
  end
end
