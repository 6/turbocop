if x
  do_something
else
  do_something_else
end
if !x
  do_something
end
x ? do_something : do_something_else
unless x
  do_something
end

unless !x
  do_something
end

# Negated condition with elsif - too complex to simply swap
if !x
  one
elsif y
  two
else
  three
end

# Elsif with negated condition should not be flagged
if x
  do_a
elsif !y
  do_b
else
  do_c
end

# Double negation should not be flagged
if !!x
  do_something
else
  do_another_thing
end

# != with multiple arguments should not be flagged
if foo.!=(bar, baz)
  do_a
else
  do_c
end

# Only part of the condition is negated
if !x && y
  do_something
else
  do_another_thing
end

# Empty else branch should not be flagged
if !condition.nil?
  foo = 42
else
end

# Both branches empty should not be flagged
if !condition.nil?
else
end

# Empty else branch in unless should not be flagged
unless !condition.nil?
  foo = 42
else
end
