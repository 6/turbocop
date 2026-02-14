f = -> x { x + 1 }
    ^^ Style/StabbyLambdaParentheses: Use parentheses for stabby lambda arguments.

g = -> x, y { x + y }
    ^^ Style/StabbyLambdaParentheses: Use parentheses for stabby lambda arguments.

h = -> a do
    ^^ Style/StabbyLambdaParentheses: Use parentheses for stabby lambda arguments.
  a * 2
end
