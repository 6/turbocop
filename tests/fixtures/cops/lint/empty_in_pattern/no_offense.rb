case condition
in [a]
  do_something
in [a, b]
  do_other_thing
end

case condition
in Integer
  handle_integer
in String
  handle_string
end

case value
in Integer
  # handle integer values
in String
  handle_string
end
