if true
^^ Lint/LiteralAsCondition: Literal `true` appeared as a condition.
  x = 1
end
while false
^^^^^ Lint/LiteralAsCondition: Literal `false` appeared as a condition.
  break
end
