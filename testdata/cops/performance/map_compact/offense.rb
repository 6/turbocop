[1, 2, 3].map { |x| x if x > 1 }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapCompact: Use `filter_map` instead of `map...compact`.
[1, 2, 3].collect { |x| x if x > 1 }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapCompact: Use `filter_map` instead of `collect...compact`.
arr.map { |item| transform(item) }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapCompact: Use `filter_map` instead of `map...compact`.
