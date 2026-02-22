f = ->(x) { x + 1 }

g = -> { puts "hello" }

obj.lambda { |x| x }

# Multi-line lambda do...end is correct in line_count_dependent style
j = lambda do |x|
  x * 2
end

k = lambda do
  puts "world"
end
