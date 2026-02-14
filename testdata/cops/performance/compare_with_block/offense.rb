arr.sort { |a, b| a.name <=> b.name }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/CompareWithBlock: Use `sort_by(&:method)` instead of `sort { |a, b| a.method <=> b.method }`.
array.sort { |a, b| a.foo <=> b.foo }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/CompareWithBlock: Use `sort_by(&:method)` instead of `sort { |a, b| a.method <=> b.method }`.
items.sort { |x, y| x.size <=> y.size }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/CompareWithBlock: Use `sort_by(&:method)` instead of `sort { |a, b| a.method <=> b.method }`.
