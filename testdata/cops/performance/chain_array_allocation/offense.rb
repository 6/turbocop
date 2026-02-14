arr.compact.map { |x| x.to_s }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/ChainArrayAllocation: Avoid chaining array methods that allocate intermediate arrays.
arr.flatten.flat_map { |x| x }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/ChainArrayAllocation: Avoid chaining array methods that allocate intermediate arrays.
arr.sort.map(&:to_s)
^^^^^^^^^^^^^^^^^^^^ Performance/ChainArrayAllocation: Avoid chaining array methods that allocate intermediate arrays.
arr.uniq.flat_map { |x| x }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/ChainArrayAllocation: Avoid chaining array methods that allocate intermediate arrays.
arr.uniq.map { |x| x.name }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/ChainArrayAllocation: Avoid chaining array methods that allocate intermediate arrays.
