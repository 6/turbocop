while true
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  work
end

until false
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  work
end

while true
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  break if done?
end

while 1
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  work
end

while 2.0
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  work
end

until nil
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  work
end

# Variable assigned inside loop but NOT referenced after — still an offense
def no_ref_after
  while true
  ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
    a = 43
    break
  end
end

# Variable assigned before loop — safe to convert, still an offense
def pre_assigned
  a = 0
  while true
  ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
    a = 42
    break
  end
  puts a
end

# Instance variable assigned inside loop — safe, still an offense
def ivar_assign
  while true
  ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
    @a = 42
    break
  end
  puts @a
end

# Nested inside an if branch — still an offense
def nested_if_branch
  if ready?
    while true
    ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
      work
      break if done?
    end
  end
end
