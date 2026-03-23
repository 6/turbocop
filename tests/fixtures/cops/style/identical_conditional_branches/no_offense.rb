# if/else with different bodies
if condition
  do_x
else
  do_y
end

# if/else with different trailing lines
if condition
  do_x
  do_z
else
  do_y
  do_w
end

# if without else
if condition
  do_x
end

# if/else with slightly different trailing lines
if something
  do_x(1)
else
  do_x(2)
end

# if/elsif without else
if something
  do_x
elsif something_else
  do_x
end

# Heredocs may look identical on the opening line but differ in content
if condition
  puts <<~MSG
    Hello
  MSG
else
  puts <<~MSG
    Goodbye
  MSG
end

# case/when without else
case something
when :a
  do_x
when :b
  do_x
end

# case/when with empty branch
case something
when :a
  do_x
  do_y
when :b
else
  do_x
  do_z
end

# case/in without else (pattern matching)
case something
in :a
  do_x
in :b
  do_x
end

# if/else leading lines — assign to condition variable
if x
  x = do_something
  foo
else
  x = do_something
  bar
end

# if/else leading lines — assign to condition receiver
if x.condition
  x = do_something
  foo
else
  x = do_something
  bar
end

# if/else leading lines — assign to condition instance variable
if @x
  @x = do_something
  foo
else
  @x = do_something
  bar
end

# if/elsif/else without complete branches (missing else)
if condition_a
  do_a
elsif condition_b
  do_same
else
  do_same
end

# case/when with one empty when branch
case value
when cond1
else
  if cond2
  else
  end
end

# case/in with one empty in branch
case value
in cond1
else
  if cond2
  else
  end
end

# if/elsif/else with identical leading lines, single child branch, last node of parent
def foo
  if something
    do_x
  elsif cond
    do_x
    x2
  else
    do_x
    x3
  end
end
