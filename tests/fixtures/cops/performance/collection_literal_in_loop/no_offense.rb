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
# Literal as receiver of enumerable method inside an outer loop
items.each do |item|
  %w[foo bar baz].each do |type|
    do_something(item, type)
  end
end
items.each do |item|
  [true, false].each do |flag|
    do_something(item, flag)
  end
end
loop do
  [1, 2, 3].each do |n|
    puts n
  end
end
items.each do |item|
  { a: 1, b: 2 }.each do |k, v|
    do_something(item, k, v)
  end
end
while true
  %w[x y z].map do |s|
    s.upcase
  end
end
