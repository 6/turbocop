def foo(bar)
  puts bar
  x = bar + 1
  x
end

def baz(x)
  x
end

def qux(name)
  result = name.upcase
  result
end

# Reassignment that references the argument on the RHS is OK
def transform(name)
  name = name.to_s.strip
  name
end

def update(value)
  value = value + 1
  value
end

# Shorthand assignments always reference the arg
def increment(count)
  count += 1
  count
end

# Assignment inside conditional -- not flagged (imprecise)
def maybe(name)
  if something?
    name = 'default'
  end
  name
end

# Argument used before reassignment
def use_first(arg)
  puts arg
  arg = 'new'
  arg
end

# Block argument with RHS reference
items.each do |item|
  item = item.to_s
  puts item
end
