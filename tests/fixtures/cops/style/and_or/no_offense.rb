if a && b
  do_something
end

if a || b
  do_something
end

while x && y
  do_something
end

x = a && b
y = a || b

# Flow control using and/or is acceptable in "conditionals" mode (the default)
foo.save and return
foo.save or raise "error"
do_something and log_it
process or abort
foo and bar
baz or qux
