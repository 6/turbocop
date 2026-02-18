foo { |a, b| a + b }
bar { |x| x }
baz { |item| item.to_s }
qux { puts 'hello' }
items.each { |i| puts i }
x = [1, 2, 3]
