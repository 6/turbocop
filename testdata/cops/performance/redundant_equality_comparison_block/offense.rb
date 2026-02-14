arr.select { |x| x == 1 }
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.any? { |item| item == value }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.reject { |x| x == "foo" }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.count { |x| x == 0 }
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
