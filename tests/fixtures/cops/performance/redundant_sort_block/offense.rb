[3, 1, 2].sort { |a, b| a <=> b }
          ^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantSortBlock: Use `sort` instead of `sort { |a, b| a <=> b }`.
arr.sort { |a, b| a <=> b }
    ^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantSortBlock: Use `sort` instead of `sort { |a, b| a <=> b }`.
items.sort { |x, y| x <=> y }
      ^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantSortBlock: Use `sort` instead of `sort { |a, b| a <=> b }`.
items.sort { _1 <=> _2 }
      ^^^^^^^^^^^^^^^^^^^^ Performance/RedundantSortBlock: Use `sort` instead of `sort { |a, b| a <=> b }`.
