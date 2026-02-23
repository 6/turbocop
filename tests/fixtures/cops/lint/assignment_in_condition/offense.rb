if x = 1
   ^^^^^ Lint/AssignmentInCondition: Assignment in condition detected. Did you mean `==`?
  do_something
end

while y = gets
      ^^^^^^^ Lint/AssignmentInCondition: Assignment in condition detected. Did you mean `==`?
  process(y)
end

until z = calculate
      ^^^^^^^^^^^^^ Lint/AssignmentInCondition: Assignment in condition detected. Did you mean `==`?
  retry_something
end
