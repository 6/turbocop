begin
  do_something
rescue ArgumentError
  handle_arg
rescue ArgumentError
       ^^^^^^^^^^^^^ Lint/DuplicateRescueException: Duplicate `rescue` exception detected.
  handle_dup
end

begin
  foo
rescue RuntimeError
  bar
rescue IOError
  baz
rescue RuntimeError
       ^^^^^^^^^^^^ Lint/DuplicateRescueException: Duplicate `rescue` exception detected.
  qux
end

begin
  a
rescue TypeError, TypeError
                  ^^^^^^^^^ Lint/DuplicateRescueException: Duplicate `rescue` exception detected.
  b
end
