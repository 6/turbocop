{foo: 1, bar: 2}.select { |k, v| k == :foo }
                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashSlice: Use `slice(:foo)` instead.
hash.select { |k, v| k == :name }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashSlice: Use `slice(:name)` instead.
hash.filter { |k, v| k == 'key' }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashSlice: Use `slice('key')` instead.
