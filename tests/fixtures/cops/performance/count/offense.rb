[1, 2, 3].select { |x| x > 1 }.count
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
[1, 2, 3].reject { |x| x > 1 }.count
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...count`.
arr.select { |item| item.valid? }.count
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
[1, 2, 3].select { |e| e.even? }.size
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...size`.
[1, 2, 3].reject { |e| e.even? }.size
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...size`.
[1, 2, 3].select { |e| e.even? }.length
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...length`.
{a: 1, b: 2}.reject { |e| e == :a }.length
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...length`.
arr.filter { |x| x > 2 }.count
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `filter...count`.
arr.find_all { |x| x > 2 }.size
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `find_all...size`.
arr.select(&:value).count
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
foo.reject(&:blank?).size
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...size`.
arr.filter(&:even?).length
^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `filter...length`.
# multi-statement block body (RuboCop does flag these)
items.map do |r|
  x = r.to_s
  r.split(".").select { |s| s == "*" }.count
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
end
