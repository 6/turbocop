array.reverse.find { |x| x > 0 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ReverseFind: Use `rfind` instead of `reverse.find`.

[1, 2, 3].reverse.find(&:odd?)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ReverseFind: Use `rfind` instead of `reverse.find`.

items.reverse.find { |i| i.valid? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ReverseFind: Use `rfind` instead of `reverse.find`.
