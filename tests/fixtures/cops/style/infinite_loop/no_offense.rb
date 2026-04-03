loop do
  work
end

loop do
  break if done?
end

while condition
  work
end

until condition
  work
end

# Variable assigned inside while true, referenced after loop — scoping exemption
def parse_sequence
  save = self.pos
  while true
    tmp = apply(:rule)
    unless tmp
      self.pos = save
      break
    end
  end
  puts tmp
end

# Variable assigned inside until false, referenced after loop
def process
  until false
    result = compute
    break if result
  end
  result
end

# Multiple variables assigned inside loop, one referenced after
def multi_var
  while true
    a, b = 42, 43
    break
  end
  puts a, b
end

# Modifier form: variable assigned inside while true, referenced after
def modifier_form
  a = next_value or break while true
  p a
end

# Nested inside an if branch with an outer reference — scoping exemption still applies
def nested_if_branch_scoping
  if ready?
    while true
      value = compute
      break if value
    end
  end
  puts value
end

# Nested inside begin/rescue with an outer reference — scoping exemption still applies
def nested_begin_scoping
  begin
    while true
      data = read_nonblock
      break unless data == :wait_readable
    end
  rescue IOError
    data = ''
  end
  puts data
end

# Keyword param modified inside loop, referenced after — scoping exemption
def kwarg_scoping(offset: 0)
  while true
    offset += 1
    break if done?
  end
  offset
end

# Variable first assigned inside a loop nested in a block is still a scoping exemption
def block_scoping_exemption
  mutex.synchronize do
    while true
      now = Time.now
      break if done?
    end

    puts now
  end
end
