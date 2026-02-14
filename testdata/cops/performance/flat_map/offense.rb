[1, 2, 3].map { |x| [x, x] }.flatten
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/FlatMap: Use `flat_map` instead of `map...flatten`.
[1, 2, 3].collect { |x| [x, x] }.flatten
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/FlatMap: Use `flat_map` instead of `collect...flatten`.