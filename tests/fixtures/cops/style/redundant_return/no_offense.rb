def foo
  42
end

def bar
  return 1 if x
  2
end

def baz
  x + 1
end

def empty
end

def multi
  a = 1
  b = 2
  a + b
end

# Guard clause (early return in middle of method)
def guard(x)
  return nil if x.nil?
  x + 1
end

# Multiple early returns
def classify(x)
  return :negative if x < 0
  return :zero if x == 0
  :positive
end

# return in non-terminal if (if is not the last statement)
def mid_method_if(x)
  if x > 10
    return :big
  end
  x + 1
end

# if/else without return in terminal position
def no_return_branches(x)
  if x > 0
    x
  else
    -x
  end
end

# case/when without return
def case_no_return(x)
  case x
  when 1 then :one
  when 2 then :two
  else :other
  end
end

# begin/rescue without return
def rescue_no_return
  begin
    do_something
  rescue
    default_value
  end
end
