a = ->(x, y) { x + y }
b = ->(x) { x * 2 }
c = -> { puts "hello" }
d = ->(a, b, c) { a + b + c }
e = lambda { |x| x + 1 }
f = ->(x) do
  x * 2
end
