# operator method — should not fire
def ==(other)
  hash == other.hash
end

# initialize — always skipped
def initialize
  @foo = true
end

# allowed method (default: call)
def call
  foo == bar
end

# unknown return in non-predicate (conservative mode) — no offense
def foo
  bar
end

# unknown return in predicate (conservative mode) — no offense
def foo?
  bar
end

# predicate with at least one boolean return (conservative mode)
def foo?
  return unless bar?
  true
end

# predicate returning boolean — correct naming
def valid?
  x == y
end

# non-predicate returning non-boolean — correct naming
def value
  5
end

# method with super return (conservative) — no offense
def check
  super
end

# method calling another method (unknown return, conservative)
def compute
  calculate_result
end

# predicate returning another predicate — correct naming
def active?
  user.active?
end

# empty body — always skipped
def placeholder
end

# bang method with unknown return (conservative) — no offense
def save!
  record.save
end

# method with multiple return values (not boolean)
def data
  return 1, 2
end

# wayward predicate — should be treated as unknown, not boolean
def status
  num.infinite?
end

# conditional with mixed returns (conservative, unknown present)
def check_something
  if condition
    true
  else
    some_method
  end
end
