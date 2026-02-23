arr.all? { |x| x == 1 }
^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.any? { |item| item == value }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.one? { |x| x == "foo" }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.none? { |x| x == 0 }
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
