if a && b
  do_something
end

if a || b
  do_something
end

while x && y
  do_something
end

# FN fix: and/or inside parentheses within conditions
until (x || y)
  do_something
end

if (a && b)
  do_something
end

do_something unless (a || b)

until (x || y || z)
  do_something
end

if foo && (bar || baz)
  do_something
end

if (a && b) || (c && d)
  do_something
end
