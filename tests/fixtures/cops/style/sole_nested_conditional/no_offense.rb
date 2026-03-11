if foo && bar
  do_something
end

if foo
  if bar
    do_something
  else
    other_thing
  end
end

if foo
  do_something
  do_other_thing
end
x = 1

# Variable assignment in outer condition used by inner condition
if var = foo
  do_something if var
end

if cond && var = foo
  do_something if var
end

if var = compute_value
  if var
    process(var)
  end
end
