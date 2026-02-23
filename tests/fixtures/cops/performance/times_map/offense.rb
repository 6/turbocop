5.times.map { |i| i * 2 }
^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/TimesMap: Use `Array.new` with a block instead of `times.map`.
10.times.map { |i| i.to_s }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/TimesMap: Use `Array.new` with a block instead of `times.map`.
n.times.map { |i| create(i) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/TimesMap: Use `Array.new` with a block instead of `times.map`.
3.times.collect { |i| i + 1 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/TimesMap: Use `Array.new` with a block instead of `times.collect`.
