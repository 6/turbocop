case condition
in [a]
  do_something
in [a, b]
^^ Lint/EmptyInPattern: Avoid `in` branches without a body.
end

case condition
in Integer
  do_something
in String
^^ Lint/EmptyInPattern: Avoid `in` branches without a body.
end

case condition
in { key: value }
^^ Lint/EmptyInPattern: Avoid `in` branches without a body.
in _
  do_something
end
