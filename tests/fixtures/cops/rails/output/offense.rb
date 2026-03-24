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
# output call inside a lambda that is an argument of a call
config.pre_term = ->(worker) { puts "Worker being killed" }
                               ^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
# output call inside a lambda do...end block
task = ->(item) do
  print item.to_s
  ^^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
end
# output call inside a rescue modifier expression
value = do_something rescue (puts "fallback")
                             ^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
# output call inside string interpolation that is a call argument
assert false, "Expected #{pp(act)} to be valid JSON"
                          ^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
# output call with splat: *p passes no-arg p result
Ironfan.noop(self, __method__, *p)
                                ^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
# output call inside else branch of case that is a call argument
puts case response
^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
     when Net::HTTPOK then "ok"
     else puts "error"
          ^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
end
# output call inside if/else that is a call argument
foo(if cond
      puts "yes"
      ^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
    else
      "no"
    end)
# output call inside a begin block that is a call argument
foo(begin; puts "x"; rescue; end)
           ^^^^ Rails/Output: Do not write to stdout. Use Rails's logger if you want to log.
