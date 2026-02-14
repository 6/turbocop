arr.sort { |a, b| a.name <=> b.name }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/CompareWithBlock: Use `sort_by(&:method)` instead of `sort { |a, b| a.method <=> b.method }`.
