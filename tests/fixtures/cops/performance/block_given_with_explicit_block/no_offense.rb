def foo(&block)
  block.call if block
end

def bar
  yield if block_given?
end

def method(x)
  do_something if block_given?
end

do_something if block_given?
