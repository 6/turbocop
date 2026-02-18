if condition
  do_x
else
  do_y
end
if condition
  do_x
  do_z
else
  do_y
  do_w
end
if condition
  do_x
end
if something
  do_x(1)
else
  do_x(2)
end
if something
  do_x
elsif something_else
  do_x
end

# elsif/else with identical last statements (only top-level if is checked)
if condition_a
  do_a
elsif condition_b
  do_same
else
  do_same
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
