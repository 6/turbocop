if foo &&
^^ Layout/EmptyLineAfterMultilineCondition: Use an empty line after a multiline condition.
   bar
  do_something
end

while foo &&
^^^^^ Layout/EmptyLineAfterMultilineCondition: Use an empty line after a multiline condition.
      bar
  do_something
end

until foo ||
^^^^^ Layout/EmptyLineAfterMultilineCondition: Use an empty line after a multiline condition.
      bar
  do_something
end
