array.sort_by { |x| x }
      ^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |x| x }`.

array.sort_by { |y| y }
      ^^^^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |y| y }`.

array.sort_by do |x|
      ^^^^^^^^^^^^^^ Style/RedundantSortBy: Use `sort` instead of `sort_by { |x| x }`.
  x
end
