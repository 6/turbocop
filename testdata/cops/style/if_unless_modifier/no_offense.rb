do_something if x

do_something unless x

if x
  do_something
else
  do_other
end

if x
  do_something
  do_other
end

unless x
  foo
  bar
end

if x
  very_long_method_name_that_would_exceed_the_max_line_length_when_used_as_a_modifier_form_together_with_the_condition
end

# elsif branches should not be flagged
if x
  do_something
elsif y
  do_other
end

if a
  one
elsif b
  two
elsif c
  three
end

# Multi-line body: can't be converted to modifier form
if condition
  method_call do
    something
  end
end

unless condition
  class Foo
    bar
  end
end
