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

def nil_in_middle?
  do_something
  nil
  do_something
end

def nil_as_arg?
  bar.baz(nil)
end

def nil_as_safe_nav_arg?
  bar&.baz(nil)
end

def nil_assigned?
  bar = nil
end

def nil_in_non_last_if?
  if bar
    true
  else
    nil
  end
  baz
end
x = 1
y = 2
