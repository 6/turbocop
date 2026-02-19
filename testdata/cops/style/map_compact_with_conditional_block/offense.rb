ary.map { |x| if x > 1 then x end }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
list.map { |item| if item.valid? then item end }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
[1, 2, 3].map { |n| if n.odd? then n end }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
