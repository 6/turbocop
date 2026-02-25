yield
yield(1, 2)
callback.call
obj.call
blk.call(arg)
# block.call inside a block, not a def — not flagged
items.each do |block|
  block.call
end
# block.call from outer scope — not flagged
around do |block|
  block.call
end

# Safe navigation — block&.call should not be replaced with yield
def process(&blk)
  blk&.call(results)
end

# block ||= reassignment means it's no longer the original block
def method(&block)
  block ||= ->(i) { puts i }
  block.call(1)
end

# block.call(&some_proc) — passing another block, can't use yield
def method(&block)
  block.call(&some_proc)
end

# block.call(1, &some_proc) — passing block along with other args
def method(&block)
  block.call(1, &some_proc)
end

# block shadowed by block variable in inner block
def method(&block)
  ->(i) { puts i }.then do |block|
    block.call(1)
  end
end
