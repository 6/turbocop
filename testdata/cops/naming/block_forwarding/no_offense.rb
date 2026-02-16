def foo(&block)
  block.call
end
def bar(&)
  baz(&)
end
def qux
  yield
end
def something(&block)
  block.arity
end
