if x
  y
end # comment
    ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `end` keyword.

begin # comment
      ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `begin` keyword.
  y
end

class X # comment
        ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `class` keyword.
  y
end

module X # comment
         ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `module` keyword.
  y
end

def x # comment
      ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.
  y
end

def x(a, b) # comment
             ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.
  y
end
