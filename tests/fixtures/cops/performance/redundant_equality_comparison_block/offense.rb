arr.all? { |x| x == 1 }
    ^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.any? { |item| item == value }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.one? { |x| x == "foo" }
    ^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.none? { |x| x == 0 }
    ^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.all? { |item| item.is_a?(String) }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.any? { |item| item.kind_of?(Integer) }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
arr.any? { |m| Pattern === m }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
# no explicit receiver (implicit self)
all? { |v| v.kind_of?(Numeric) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
any? { |obj| obj.is_a?(Klass) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
any? { |x| Pattern === x }
^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
# param used as receiver chain on other side (not as method argument)
items.all? { |k| k == k.to_i.to_s }
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantEqualityComparisonBlock: Use `grep` or `===` comparison instead of block with `==`.
