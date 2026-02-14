x.map(&:foo).map(&:bar)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/MapMethodChain: Use `map` with a block instead of chaining multiple `map` calls with symbol arguments.
