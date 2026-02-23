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
