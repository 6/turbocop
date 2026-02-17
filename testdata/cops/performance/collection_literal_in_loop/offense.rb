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
