x = if condition
  1
else
  2
end

if condition
  x = 1
else
  y = 2
end

if condition
  do_something
else
  do_other_thing
end

# elsif branches should not be flagged even if they look like simple if/else
if condition_a
  x = 1
elsif condition_b
  x = 2
else
  x = 3
end
