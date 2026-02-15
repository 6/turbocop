f = ->(x) { x + 1 }

g = -> { puts "hello" }

h = ->(x) do
  x * 2
end

obj.lambda { |x| x }

i = -> do
  puts "world"
end

# Multi-line lambda do...end is correct in line_count_dependent style
j = lambda do |x|
  x * 2
end

k = lambda do
  puts "world"
end
