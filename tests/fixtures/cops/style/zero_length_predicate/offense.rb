[1, 2, 3].length == 0
^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `[1, 2, 3].length == 0`.

'foobar'.length == 0
^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `'foobar'.length == 0`.

array.size == 0
^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `array.size == 0`.

hash.size > 0
^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `hash.size > 0`.

Post.find_all.length > 0
^^^^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `Post.find_all.length > 0`.

Animal.db_indexes.size > 0
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `Animal.db_indexes.size > 0`.

Object.methods.length > 0
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `Object.methods.length > 0`.

ENV['FOFA_INVALID_IP'].size > 0
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `ENV['FOFA_INVALID_IP'].size > 0`.

parameters&.length == 0
^^^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `parameters&.length == 0`.
