[1, 2, 3].select { |x| x > 1 }.map { |x| x * 2 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SelectMap: Use `filter_map` instead of `select...map`.
[1, 2, 3].filter { |x| x > 1 }.map { |x| x * 2 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/SelectMap: Use `filter_map` instead of `filter...map`.