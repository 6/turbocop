while node
  do_something(node)
  node = node.parent
end

items.each do |item|
  if something?(item)
    return item
  end
end

loop do
  break if done?
  do_something
end

# Conditional next prevents false positive
while x > 0
  next if x.odd?
  x += 1
  break
end

# next unless (modifier form)
items.each do |key|
  if value.blank?
    abort "missing"
  end
  next unless condition?
  abort "bad value"
end

# redo if (modifier form)
while x > 0
  redo if x.odd?
  case x
  when 1
    break
  else
    raise MyError
  end
end

# next inside if branch
while x > 0
  if y
    next if something
    break
  else
    break
  end
end

# next inside rescue clause
available_socks.each do |sock|
  begin
    sock.connect_nonblock(addr)
  rescue => e
    sock.close
    next
  end
  return sock
end

# begin/rescue where rescue can fall through (not all paths break)
loop do
  begin
    connection.establish
    break
  rescue => e
    prompt.error e.message
    unless prompt.yes?('Try again?')
      break
    end
  end
end

# Multiple next guards before return (common Ruby pattern)
names.each do |name|
  next if cop_names.include?(name)
  next if departments.include?(name)
  next if SYNTAX_DEPARTMENTS.include?(name)
  raise IncorrectCopNameError
end

# next unless guard before return
tokens.each do |token|
  next unless token.match?(/pattern/)
  return token.downcase
end

# if without else (not all branches break)
while x > 0
  if condition
    break
  elsif other_condition
    raise MyError
  end
end

# case without else (not all branches break)
while x > 0
  case x
  when 1
    break
  end
end

# case-when-else with not all branches breaking
while x > 0
  case x
  when 1
    break
  when 2
    do_something
  else
    raise MyError
  end
end
