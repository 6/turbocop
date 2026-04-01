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

# While-as-expression (natalie pattern): while true used as RHS of assignment
a = while true; break 1; end
    ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.

# while true inside begin block
def inside_begin
  begin
    while true
    ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
      work
      break if done?
    end
  rescue => e
    handle(e)
  end
end

# while true inside a do..end block (e.g. Thread.new)
def inside_block
  Thread.new do
    while true
    ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
      work
      break if done?
    end
  end
end

# while true with do keyword
while true do
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  work
  break if done?
end

# Variable with same name as block-local var should not suppress offense
def block_local_no_suppress
  while true
  ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
    items.each do |line|
      process(line)
    end
    break if done?
  end
end

# Keyword param modified inside loop, not referenced after — still offense
def kwarg_no_ref(offset: 0)
  while true
  ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
    offset += 1
    break if done?
  end
end
