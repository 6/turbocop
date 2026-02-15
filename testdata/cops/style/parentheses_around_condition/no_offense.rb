if x > 1
  do_something
end

while x > 1
  do_something
end

until x > 1
  do_something
end

x > 1 ? a : b

if x && y
  do_something
end

# AllowSafeAssignment: true (default)
if (a = something)
  use(a)
end

while (line = gets)
  process(line)
end

if (result = compute)
  handle(result)
end
