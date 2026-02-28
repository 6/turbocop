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

# Anonymous block forwarding (Ruby 3.1+) — not an offense
def method_with_anon_block(x, &)
  raise ArgumentError, "block required" unless block_given?
  do_something(&)
end

def relation(name, options = {}, &)
  klass.class_eval(&) if block_given?
end

def plugin(adapter, spec, &)
  block_given? ? yield_config : default_config
end
