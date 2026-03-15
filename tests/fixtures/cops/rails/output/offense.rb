puts "hello"
^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
print "hello"
^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
p value
^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
pp object
^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
ap record
^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
pretty_print item
^^^^^^^^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
$stdout.write "data"
^^^^^^^^^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
$stderr.syswrite "data"
^^^^^^^^^^^^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
STDOUT.binwrite "data"
^^^^^^^^^^^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
STDERR.write_nonblock "data"
^^^^^^^^^^^^^^^^^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
::STDOUT.write "data"
^^^^^^^^^^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
::STDERR.write "data"
^^^^^^^^^^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
print
^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
pp
^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
puts
^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
$stdout.write
^^^^^^^^^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
STDERR.write
^^^^^^^^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
# output call inside a block that is an argument of another call
bar(foo { puts "hello" })
          ^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
something.map { |x| print x }
                    ^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
items.each { |i| p i }
                 ^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
# deeply nested: block inside argument inside block inside argument
formatter = proc do |msg|
  msg.tap { |m| puts m }
                ^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
end
