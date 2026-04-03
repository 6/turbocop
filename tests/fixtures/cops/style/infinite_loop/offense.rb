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

# Later block parameters with the same name should not suppress offense
def shadowed_block_param_after_loop
  while true
  ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
    event = next_value
    break if done?
  end

  handlers.each do |event, funcname|
    process(event, funcname)
  end
end

# Outer local assigned before a block remains visible inside nested loop blocks
def block_outer_local_assigned_before
  now = nil

  mutex.synchronize do
    while true
    ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
      now = Time.now
      break if done?
    end

    puts now
  end
end

# Outer block locals remain visible inside inner blocks
def nested_block_outer_local_assigned_before
  active.each do |zipper|
    matched = false

    groups.each do |group|
      while true
      ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
        matched = compute(zipper, group)
        break if matched
      end

      break if matched
    end
  end
end

# Outer locals assigned before a parameterized block remain visible inside the loop
def parametrized_block_outer_local_assigned_before
  out = 0

  input_length.times do |j|
    while true
    ^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
      out += 1
      break if j.zero?
    end

    puts out
  end
end

# Backtick commands are truthy literals
while `isDesc ? #{counter >= limit} : #{counter <= limit}`
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  counter += step
end
