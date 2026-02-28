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

# Block param reassignment — not an offense (RuboCop suppresses)
def with_default_block(&block)
  block ||= -> { default_action }
  block.call if block_given?
end

def with_reassigned_block(&block)
  block = proc { fallback } unless block_given?
  block.call
end

def with_and_assign(&block)
  block &&= wrap(block)
  block.call if block_given?
end
