[1, 2, 3].select { |x| x > 1 }.first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select.first`.
arr.select { |item| item.valid? }.first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select.first`.
users.select { |u| u.active? }.first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select.first`.
