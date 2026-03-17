a = ->(x, y) { x + y }
b = ->(x) { x * 2 }
c = -> { puts "hello" }
d = ->(a, b, c) { a + b + c }
e = lambda { |x| x + 1 }
f = ->(x) do
  x * 2
end
# Empty parameter lists should not be flagged (RuboCop skips them)
g = -> () { puts "hello" }
h = -> () do
  puts "world"
end
# Non-parenthesized params with no space are fine
i = ->x { x + 1 }
j = ->x, y { x + y }
k = ->*args { args }
