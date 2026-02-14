def foo(&block)
  if block_given?
     ^^^^^^^^^^^^^ Performance/BlockGivenWithExplicitBlock: Check `block` instead of using `block_given?` with explicit `&block` parameter.
    block.call
  end
end
