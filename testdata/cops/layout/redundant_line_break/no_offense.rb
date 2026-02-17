my_method(1, 2, "x")

foo(a, b)

a = if x
      1
    else
      2
    end

foo \
  && bar

foo \
  || bar

x = 42

# Backslash in a comment line should not trigger
# 'foo' \
#   'bar'

# This is a YARD example with backslash \
# continuation that is just a comment

# A line that would be too long when combined (exceeds 120 chars):
this_is_a_very_long_method_name_that_makes_the_line_quite_long(argument_one, argument_two, argument_three) \
  .and_then_another_long_chain_call

MSG = 'This is a long error message string that definitely ' \
      'exceeds one hundred and twenty characters when concatenated together'
