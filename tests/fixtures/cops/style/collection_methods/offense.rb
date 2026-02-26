[1, 2, 3].collect { |e| e + 1 }
          ^^^^^^^ Style/CollectionMethods: Prefer `map` over `collect`.

[1, 2, 3].inject { |a, e| a + e }
          ^^^^^^ Style/CollectionMethods: Prefer `reduce` over `inject`.

[1, 2, 3].detect { |e| e > 1 }
          ^^^^^^ Style/CollectionMethods: Prefer `find` over `detect`.

# Block pass (&:sym) should also be flagged
[1, 2, 3].collect(&:to_s)
          ^^^^^^^ Style/CollectionMethods: Prefer `map` over `collect`.

[1, 2, 3].detect(&:odd?)
          ^^^^^^ Style/CollectionMethods: Prefer `find` over `detect`.

# Symbol arg for MethodsAcceptingSymbol methods
[1, 2, 3].inject(:+)
          ^^^^^^ Style/CollectionMethods: Prefer `reduce` over `inject`.

[1, 2, 3].inject(0, :+)
          ^^^^^^ Style/CollectionMethods: Prefer `reduce` over `inject`.

# member? with a block should be flagged
[1, 2, 3].member? { |e| e > 1 }
          ^^^^^^^ Style/CollectionMethods: Prefer `include?` over `member?`.
