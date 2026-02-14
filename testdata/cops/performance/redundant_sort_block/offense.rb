[3, 1, 2].sort { |a, b| a <=> b }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantSortBlock: Use `sort` instead of `sort { |a, b| a <=> b }`.