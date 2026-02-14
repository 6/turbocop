[1, 2, 3].inject(:+)
^^^^^^^^^^^^^^^^^^^^ Performance/Sum: Use `sum` instead of `inject(:+)`.
[1, 2, 3].inject(0, :+)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/Sum: Use `sum` instead of `inject(0, :+)`.
[1, 2, 3].reduce(:+)
^^^^^^^^^^^^^^^^^^^^ Performance/Sum: Use `sum` instead of `reduce(:+)`.
[1, 2, 3].reduce(0, :+)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/Sum: Use `sum` instead of `reduce(0, :+)`.