{foo: 1, bar: 2, baz: 3}.reject { |k, v| k == :bar }
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:bar)` instead.
{foo: 1, bar: 2, baz: 3}.select { |k, v| k != :bar }
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:bar)` instead.
{foo: 1, bar: 2, baz: 3}.filter { |k, v| k != :bar }
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:bar)` instead.
hash.reject { |k, v| k == 'str' }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except('str')` instead.
hash.reject { |k, _| [:foo, :bar].include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:foo, :bar)` instead.
hash.reject { |k, _| KEYS.include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(*KEYS)` instead.
hash.select { |k, _| !KEYS.include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(*KEYS)` instead.
hash.filter { |k, _| !KEYS.include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(*KEYS)` instead.
hash.reject { |k, _| excluded.include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(*excluded)` instead.
hash.reject do |k, _|
     ^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(*excluded)` instead.
  excluded.include?(k)
end
