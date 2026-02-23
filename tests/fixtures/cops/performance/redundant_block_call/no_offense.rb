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
