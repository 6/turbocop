[1, 2, 3].map { |x| x if x > 1 }.compact
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapCompact: Use `filter_map` instead.
[1, 2, 3].collect { |x| x if x > 1 }.compact
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapCompact: Use `filter_map` instead.
arr.map { |item| transform(item) }.compact
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapCompact: Use `filter_map` instead.
collection.map(&:do_something).compact
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapCompact: Use `filter_map` instead.
collection.collect(&:do_something).compact
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapCompact: Use `filter_map` instead.
items.collect {|x|
      ^^^^^^^^^^^^ Performance/MapCompact: Use `filter_map` instead.
  x.valid? ? x.name : nil
}.compact
