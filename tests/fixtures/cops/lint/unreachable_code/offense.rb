def foo
  return 1
  puts 'after return'
  ^^^^ Lint/UnreachableCode: Unreachable code detected.
end

def bar
  raise 'error'
  cleanup
  ^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
end

def baz
  fail 'error'
  do_something
  ^^^^^^^^^^^^ Lint/UnreachableCode: Unreachable code detected.
end
