if x = 42
   ^^^^^^ Lint/LiteralAssignmentInCondition: Don't use literal assignment `= 42` in conditional, should be `==` or non-literal operand.
  do_something
end

if y = true
   ^^^^^^^^ Lint/LiteralAssignmentInCondition: Don't use literal assignment `= true` in conditional, should be `==` or non-literal operand.
  do_something
end

while z = "hello"
      ^^^^^^^^^^^ Lint/LiteralAssignmentInCondition: Don't use literal assignment `= "hello"` in conditional, should be `==` or non-literal operand.
  do_something
end
