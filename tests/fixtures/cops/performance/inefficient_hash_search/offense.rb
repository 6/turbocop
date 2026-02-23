hash.keys.include?(:foo)
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/InefficientHashSearch: Use `key?` instead of `keys.include?`.
hash.values.include?("bar")
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/InefficientHashSearch: Use `value?` instead of `values.include?`.
{a: 1}.keys.include?(:a)
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/InefficientHashSearch: Use `key?` instead of `keys.include?`.
