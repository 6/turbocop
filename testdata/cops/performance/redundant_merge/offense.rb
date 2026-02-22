hash.merge!(a: 1)
^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
hash.merge!(key: value)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
opts.merge!(debug: true)
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
h = {}
h.merge!(a: 1, b: 2)
^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with 2 key-value pairs.
puts "done"
settings = {}
settings.merge!(Port: port, Host: bind)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with 2 key-value pairs.
start_server
