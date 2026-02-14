hash.merge!(a: 1)
^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
hash.merge!(key: value)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
opts.merge!(debug: true)
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
