def foo
  return 1
end

def bar
  raise 'error' if condition
  do_something
end

def baz
  if condition
    return 1
  end
  2
end
