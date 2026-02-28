[1, 2, 3].reverse.first
          ^^^^^^^^^^^^^ Performance/ReverseFirst: Use `last` instead of `reverse.first`.
[1, 2, 3].reverse.first(2)
          ^^^^^^^^^^^^^^^^ Performance/ReverseFirst: Use `last(2).reverse` instead of `reverse.first(2)`.
arr.reverse.first
    ^^^^^^^^^^^^^ Performance/ReverseFirst: Use `last` instead of `reverse.first`.
