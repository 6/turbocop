def foo(a, b)
  super
end

def bar(x, y)
  super(x)
end

def baz(a, b)
  super(b, a)
end
x = 1
y = 2

# Forwarding only a subset of args
def with_kwargs(a, b:)
  super(a)
end

# Different rest args
def with_rest(a, *args)
  super(a)
end

# Different block
def with_block(a, &blk)
  super(a)
end

# Anonymous keyword rest — super doesn't forward keyword args
def initialize(app, **)
  super app
end

# Super inside a block — should not be flagged
def perform(*args)
  some_block do
    super(*args)
  end
end

# Super inside a proc/lambda
def execute(sql, options, &block)
  trace_execute(proc { super(sql, options, &block) })
end

# Super in define_singleton_method block
def test(a)
  define_singleton_method(:test2) do |a|
    super(a)
  end
end

# Super with different block argument name
def initialize(*args, &task)
  traced = add_tracing(&task)
  super(*args, &traced)
end

# Super with reassigned block argument
def test(&blk)
  blk = proc {}
  super(&blk)
end

# Keyword arguments in different order (no offense)
def reorder(a:, b:)
  super(b: b, a: a)
end

# Explicitly passing no arguments (super() with arg def)
def explicit_empty(a)
  super()
end

# super inside Class.new block — scope changes
def foo(a)
  Class.new do
    def foo(a, b)
      super(a)
    end
  end
end

# DSL method with super()
describe 'example' do
  subject { super() }
end

# Anonymous block param (&) — super has different forwarding semantics
def create(promise = nil, &)
  super(promise)
end

# Mixed anonymous forwarding should not flag when only a subset is forwarded
def partial_anonymous_forwarding(*, **)
  super(*)
end

def partial_anonymous_keyword_forwarding(*, **)
  super(**)
end
