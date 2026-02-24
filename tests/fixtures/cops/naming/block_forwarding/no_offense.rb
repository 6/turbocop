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
# block used in complex expression: &(block || fallback)
def with_or_fallback(&block)
  request(&(block || @default_block))
end
# block used in complex expression: &(proc || block)
def column(proc = nil, &block)
  table.add(&(proc || block))
end
# block used in ternary expression: &(cond ? block : other)
def conditional_forward(&block)
  run(&(flag ? block : fallback))
end
# block param reassigned via multi-assignment (destructuring)
def multi_assign(&task)
  *args, task = wrapper.call(&task)
  executor.post(*args, &task)
end
# block param reassigned via tuple destructuring
def tuple_assign(&block)
  name, block = items.first
  bar(&block)
end
# block param reassigned inside nested block via multi-assignment
def nested_multi_assign(&block)
  items.each do |item|
    a, block = item
  end
  bar(&block)
end
