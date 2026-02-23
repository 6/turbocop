x -= 1 while x > 0
process_next until done
while x > 0
  x -= 1
  puts x
end
until done
  step1
  step2
end
x = 10
while some_very_long_condition_name && another_very_long_condition_name && yet_another_very_long_condition_name
  do_something_with_a_very_long_body_that_exceeds_line_length
end
while (chunk = file.read(1024))
  io.write(chunk)
end
while (node = node.receiver)
  return true if node.heredoc?
end
