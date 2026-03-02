def foo(...)
  bar(...)
end

def baz(x, y)
  qux(x, y)
end

def test
  42
end

# Non-redundant names: *items and &handler are NOT in the default redundant lists
# So neither anonymous forwarding nor ... forwarding applies
def self.with(*items, &handler)
  new(*items).tap(&handler).to_element
end

# Non-redundant block and rest names — no forwarding suggestions
def process(*entries, &callback)
  entries.each(&callback)
end

# Both args referenced directly — no anonymous forwarding possible
def capture(*args, &block)
  args.each { |a| puts a }
  block.call
  run(*args, &block)
end

# No body — nothing to forward to
def empty(*args, &block)
end
