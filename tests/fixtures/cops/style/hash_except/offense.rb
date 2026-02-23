{foo: 1, bar: 2, baz: 3}.reject { |k, v| k == :bar }
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:bar)` instead.
{foo: 1, bar: 2, baz: 3}.select { |k, v| k != :bar }
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:bar)` instead.
{foo: 1, bar: 2, baz: 3}.filter { |k, v| k != :bar }
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:bar)` instead.
hash.reject { |k, v| k == 'str' }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except('str')` instead.
