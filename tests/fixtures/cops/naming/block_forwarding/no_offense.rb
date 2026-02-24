# block used as variable (call)
def foo(&block)
  block.call
end
# already anonymous
def bar(&)
  baz(&)
end
# no block param at all
def qux
  yield
end
# block used as variable (arity)
def something(&block)
  block.arity
end
# block used as variable (returned)
def returns_block(&block)
  block
end
# block used as condition
def uses_block_cond(&block)
  bar(&block) if block
end
# keyword param (Ruby 3.1 syntax error with anonymous &)
def with_kwarg(k:, &block)
  bar(&block)
end
# optional keyword param
def with_kwoptarg(k: 42, &block)
  bar(&block)
end
# block param assigned
def reassigned(&block)
  block ||= -> { :foo }
  bar(&block)
end
# no arguments method
def no_args
end
