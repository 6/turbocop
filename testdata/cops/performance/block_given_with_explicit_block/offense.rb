def foo(&block)
  if block_given?
     ^^^^^^^^^^^^^ Performance/BlockGivenWithExplicitBlock: Check `block` instead of using `block_given?` with explicit `&block` parameter.
    block.call
  end
end
def method(x, &block)
  do_something if block_given?
                  ^^^^^^^^^^^^^ Performance/BlockGivenWithExplicitBlock: Check `block` instead of using `block_given?` with explicit `&block` parameter.
end
def self.method(x, &block)
  do_something if block_given?
                  ^^^^^^^^^^^^^ Performance/BlockGivenWithExplicitBlock: Check `block` instead of using `block_given?` with explicit `&block` parameter.
end
def method(x, &myblock)
  do_something if block_given?
                  ^^^^^^^^^^^^^ Performance/BlockGivenWithExplicitBlock: Check `block` instead of using `block_given?` with explicit `&block` parameter.
end
