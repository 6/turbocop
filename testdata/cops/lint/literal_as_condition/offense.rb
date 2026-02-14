if true
^^ Lint/LiteralAsCondition: Literal `true` appeared as a condition.
  x = 1
end
if 42
^^ Lint/LiteralAsCondition: Literal `42` appeared as a condition.
  x = 2
end
while false
^^^^^ Lint/LiteralAsCondition: Literal `false` appeared as a condition.
  break
end
