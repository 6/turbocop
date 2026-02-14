def foo(&block)
  block.call if block
end

def bar
  yield if block_given?
end
