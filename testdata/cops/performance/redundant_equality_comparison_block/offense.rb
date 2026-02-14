arr.select { |x| x == 1 }
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.any? { |item| item == value }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
