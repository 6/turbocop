case x
in 'first'
  do_something
in 'first'
   ^^^^^^^ Lint/DuplicateMatchPattern: Duplicate `in` pattern detected.
  do_something_else
end

case x
in 1
  do_something
in 1
   ^ Lint/DuplicateMatchPattern: Duplicate `in` pattern detected.
  do_something_else
end

case x
in :foo
  do_something
in :foo
   ^^^^ Lint/DuplicateMatchPattern: Duplicate `in` pattern detected.
  do_something_else
end
