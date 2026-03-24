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

# block.call with &block_pass in one occurrence suppresses ALL occurrences
def method(&block)
  block.call(1, &some_proc)
  block.call(2)
end

# block.call with block literal
def method(&block)
  block.call { do_something }
end

# block.call with numbered block argument
def method(&block)
  block.call { _1.do_something }
end

# block reassigned via multi-write / destructuring
def system(*cmd, &block)
  exe, pars, printable, block = prepare_command(cmd, &block)
  block.call(1, 2, 3)
end
