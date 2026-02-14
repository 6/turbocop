f = ->(x) { x + 1 }

g = -> { puts "hello" }

h = ->(x) do
  x * 2
end

obj.lambda { |x| x }

i = -> do
  puts "world"
end
