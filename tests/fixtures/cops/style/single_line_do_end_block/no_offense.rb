foo { |x| x }
bar { puts 'hello' }
baz do |x|
  x + 1
end
qux do
  puts 'hello'
end
x = [1, 2, 3]

years = (years.is_a?(Array) ? years : [years])
        .sort_by do |x|
          x
        end
