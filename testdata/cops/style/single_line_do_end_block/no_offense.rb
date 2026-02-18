foo { |x| x }
bar { puts 'hello' }
baz do |x|
  x + 1
end
qux do
  puts 'hello'
end
x = [1, 2, 3]
