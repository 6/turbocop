[1, 2, 3].select { |x| x > 1 }.count
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
[1, 2, 3].reject { |x| x > 1 }.count
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...count`.
arr.select { |item| item.valid? }.count
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
