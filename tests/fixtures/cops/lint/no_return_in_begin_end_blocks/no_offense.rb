@some_variable ||= begin
  if some_condition_is_met
    some_value
  else
    do_something
  end
end

x = if condition
  return 1
end

some_value += begin
  if rand(1..2).odd?
    "odd number"
  else
    "even number"
  end
end

some_value -= begin
  2
end

# And-assignments without return are fine
x = 1
x &&= begin
  42
end

@ivar &&= begin
  42
end

$gvar &&= begin
  42
end

$gvar ||= begin
  42
end

# Method call assignments without return are fine
obj = Object.new
obj.foo &&= begin
  42
end

obj.foo ||= begin
  42
end

obj.foo += begin
  42
end

# Index assignments without return are fine
arr = [1, 2, 3]
arr[0] &&= begin
  42
end

arr[0] ||= begin
  42
end

arr[0] += begin
  42
end

# return inside a method but NOT in a begin..end assignment
def normal_method
  return if invalid?
  x = begin
    compute
  end
end
