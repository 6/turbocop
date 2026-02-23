[1, 2, 3].length == 0
^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `[1, 2, 3].length == 0`.

'foobar'.length == 0
^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `'foobar'.length == 0`.

array.size == 0
^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `array.size == 0`.

hash.size > 0
^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `hash.size > 0`.
