def foo?
  return true if condition
  false
end

def bar?
  condition && other_condition
end

def baz
  return nil
end

def qux?
  do_something

  nil

  do_something
end

def quux?
  if bar
    true
  else
    nil
  end
  baz
end

x = 1
y = 2
