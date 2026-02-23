hash[foo.object_id] = :bar
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/HashCompareByIdentity: Use `Hash#compare_by_identity` instead of using `object_id` for keys.

hash.key?(baz.object_id)
^^^^^^^^^^^^^^^^^^^^^^^^ Lint/HashCompareByIdentity: Use `Hash#compare_by_identity` instead of using `object_id` for keys.

hash.fetch(x.object_id)
^^^^^^^^^^^^^^^^^^^^^^^ Lint/HashCompareByIdentity: Use `Hash#compare_by_identity` instead of using `object_id` for keys.
