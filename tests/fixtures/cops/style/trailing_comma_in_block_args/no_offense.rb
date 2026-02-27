foo { |a, b| a + b }
bar { |x| x }
baz { |item| item.to_s }
qux { puts 'hello' }
items.each { |i| puts i }
x = [1, 2, 3]

# Single parameter with trailing comma (destructuring) — NOT an offense
items.each { |item,| process(item) }
hash.each_key { |key,| keys << key }
[1, 2].map { |x, | x * 2 }
test { |a,| a }
test do |a,|
  a
end
define_method(:m) { |a,| a }
