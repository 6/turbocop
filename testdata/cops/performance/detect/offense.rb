[1, 2, 3].select { |x| x > 1 }.first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select.first`.