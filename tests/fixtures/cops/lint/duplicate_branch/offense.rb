if condition
  do_something
elsif other
^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_something
end

case x
when 1
  :foo
when 2
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  :foo
when 3
  :bar
end

case y
when :a
  handle_a
when :b
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  handle_a
when :c
  handle_c
end
