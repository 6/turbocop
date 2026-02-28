[1, 2, 3].select { |x| x > 1 }.first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select.first`.
arr.select { |item| item.valid? }.first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select.first`.
users.select { |u| u.active? }.first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select.first`.
[1, 2, 3].select { |x| x > 1 }.last
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `reverse.detect` instead of `select.last`.
items.select { |i| i.ready? }.last
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `reverse.detect` instead of `select.last`.
[1, 2, 3].find_all { |x| x > 1 }.first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `find_all.first`.
arr.find_all { |item| item.ok? }.last
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `reverse.detect` instead of `find_all.last`.
[1, 2, 3].filter { |x| x > 1 }.first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `filter.first`.
arr.filter { |item| item.ok? }.last
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `reverse.detect` instead of `filter.last`.
[1, 2, 3].select(&:even?).first
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select.first`.
[1, 2, 3].select(&:even?).last
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `reverse.detect` instead of `select.last`.
[1, 2, 3].select { |i| i % 2 == 0 }[0]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select[0]`.
[1, 2, 3].select { |i| i % 2 == 0 }[-1]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `reverse.detect` instead of `select[-1]`.
[1, 2, 3].select(&:even?)[0]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select[0]`.
[1, 2, 3].select(&:even?)[-1]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `reverse.detect` instead of `select[-1]`.
[1, 2, 3].select do |i|
^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select.first`.
  i % 2 == 0
end.first
lazy.select(&:even?).first
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Detect: Use `detect` instead of `select.first`.
