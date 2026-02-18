hash.fetch('foo', nil)&.fetch('bar', nil)
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashFetchChain: Use `dig('foo', 'bar')` instead.
hash.fetch(:foo, {})&.fetch(:bar, nil)
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashFetchChain: Use `dig(:foo, :bar)` instead.
hash.fetch('a', Hash.new)&.fetch('b', nil)
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashFetchChain: Use `dig('a', 'b')` instead.
