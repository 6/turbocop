x.map(&:foo).map(&:bar)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapMethodChain: Use `map` with a block instead of chaining multiple `map` calls with symbol arguments.
arr.map(&:to_s).map(&:upcase)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapMethodChain: Use `map` with a block instead of chaining multiple `map` calls with symbol arguments.
items.map(&:name).map(&:downcase)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapMethodChain: Use `map` with a block instead of chaining multiple `map` calls with symbol arguments.
