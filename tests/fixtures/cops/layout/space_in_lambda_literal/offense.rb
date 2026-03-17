a = -> (x, y) { x + y }
      ^ Layout/SpaceInLambdaLiteral: Do not use spaces between `->` and `(` in lambda literals.
b = -> (x) { x * 2 }
      ^ Layout/SpaceInLambdaLiteral: Do not use spaces between `->` and `(` in lambda literals.
c = -> (a, b, c) { a + b + c }
      ^ Layout/SpaceInLambdaLiteral: Do not use spaces between `->` and `(` in lambda literals.
d = -> x { x + 1 }
      ^ Layout/SpaceInLambdaLiteral: Do not use spaces between `->` and `(` in lambda literals.
e = -> x, y { x + y }
      ^ Layout/SpaceInLambdaLiteral: Do not use spaces between `->` and `(` in lambda literals.
f = -> x { -> y { x + y } }
      ^ Layout/SpaceInLambdaLiteral: Do not use spaces between `->` and `(` in lambda literals.
             ^ Layout/SpaceInLambdaLiteral: Do not use spaces between `->` and `(` in lambda literals.
g = -> *args { args }
      ^ Layout/SpaceInLambdaLiteral: Do not use spaces between `->` and `(` in lambda literals.
