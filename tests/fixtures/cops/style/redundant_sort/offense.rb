[2, 1, 3].sort.first
          ^^^^^^^^^^ Style/RedundantSort: Use `min` instead of `sort...first`.

[2, 1, 3].sort.last
          ^^^^^^^^^ Style/RedundantSort: Use `max` instead of `sort...last`.

[2, 1, 3].sort[0]
          ^^^^^^^ Style/RedundantSort: Use `min` instead of `sort...[0]`.

[2, 1, 3].sort[-1]
          ^^^^^^^^ Style/RedundantSort: Use `max` instead of `sort...[-1]`.

[2, 1, 3].sort.at(0)
          ^^^^^^^^^^^ Style/RedundantSort: Use `min` instead of `sort...at(0)`.

[2, 1, 3].sort.slice(-1)
          ^^^^^^^^^^^^^^^ Style/RedundantSort: Use `max` instead of `sort...slice(-1)`.

foo.sort_by { |x| x.length }.first
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSort: Use `min_by` instead of `sort_by...first`.

foo.sort_by { |x| x.length }.last
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSort: Use `max_by` instead of `sort_by...last`.

foo.sort_by(&:name).first
    ^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSort: Use `min_by` instead of `sort_by...first`.

foo.sort_by(&:name).last
    ^^^^^^^^^^^^^^^^^^^^ Style/RedundantSort: Use `max_by` instead of `sort_by...last`.

foo.sort { |a, b| b <=> a }.last
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSort: Use `max` instead of `sort...last`.

foo.sort { |a, b| a <=> b }.first
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSort: Use `min` instead of `sort...first`.

items.sort { |a, b| a.name <=> b.name }[0]
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSort: Use `min` instead of `sort...[0]`.

items
  .sort_by { |x| x.name }
   ^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSort: Use `min_by` instead of `sort_by...first`.
  .first

items
  .sort_by { |x| x.name }
   ^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSort: Use `max_by` instead of `sort_by...last`.
  .last

items
  .sort { |a, b| a.score <=> b.score }
   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSort: Use `max` instead of `sort...last`.
  .last
