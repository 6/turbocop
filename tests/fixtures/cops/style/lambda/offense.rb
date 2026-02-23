f = lambda { |x| x + 1 }
    ^^^^^^ Style/Lambda: Use the `-> {}` lambda literal syntax for single-line lambdas.

g = lambda { puts "hello" }
    ^^^^^^ Style/Lambda: Use the `-> {}` lambda literal syntax for single-line lambdas.

h = lambda { |a, b| a + b }
    ^^^^^^ Style/Lambda: Use the `-> {}` lambda literal syntax for single-line lambdas.

j = ->() do
    ^^ Style/Lambda: Use the `lambda` method for multiline lambdas.
  something
end

k = -> do
    ^^ Style/Lambda: Use the `lambda` method for multiline lambdas.
  if condition
    action
  end
end
