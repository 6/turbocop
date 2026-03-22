[3, 1, 2].sort { |a, b| b <=> a }
          ^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SortReverse: Use `sort.reverse` instead of `sort { |a, b| b <=> a }`.
arr.sort { |x, y| y <=> x }
    ^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SortReverse: Use `sort.reverse` instead of `sort { |a, b| b <=> a }`.
items.sort { |left, right| right <=> left }
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SortReverse: Use `sort.reverse` instead of `sort { |a, b| b <=> a }`.
# multiline chained receiver — offense should be at .sort, not receiver start
items.select { |x| x > 0 }.sort { |a, b| b <=> a }
                           ^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SortReverse: Use `sort.reverse` instead of `sort { |a, b| b <=> a }`.
