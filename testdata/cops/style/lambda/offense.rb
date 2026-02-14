f = lambda { |x| x + 1 }
    ^^^^^^ Style/Lambda: Use the `-> {}` lambda literal syntax for all lambdas.

g = lambda { puts "hello" }
    ^^^^^^ Style/Lambda: Use the `-> {}` lambda literal syntax for all lambdas.

h = lambda do |x|
    ^^^^^^ Style/Lambda: Use the `-> {}` lambda literal syntax for all lambdas.
  x * 2
end

i = lambda do
    ^^^^^^ Style/Lambda: Use the `-> {}` lambda literal syntax for all lambdas.
  puts "world"
end
