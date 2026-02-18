[1, 2, 3].collect { |e| e + 1 }
          ^^^^^^^ Style/CollectionMethods: Prefer `map` over `collect`.

[1, 2, 3].inject { |a, e| a + e }
          ^^^^^^ Style/CollectionMethods: Prefer `reduce` over `inject`.

[1, 2, 3].detect { |e| e > 1 }
          ^^^^^^ Style/CollectionMethods: Prefer `find` over `detect`.
