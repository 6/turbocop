def foo(bar, baz)
end

def method_name(arg1, arg2, arg3)
end

def another(a, b)
  a + b
end

def no_params
end

# Multiline signature that would exceed max line length if joined (no offense)
def method_with_many_params(first_parameter, second_parameter, third_parameter, fourth_parameter,
                            fifth_parameter, sixth_parameter)
  first_parameter
end

def a_very_long_method_name_that_takes_up_space(parameter_one_name, parameter_two_name, parameter_three_name,
                                                 parameter_four_name)
  parameter_one_name
end

# Multiline without explicit parens (no offense per RuboCop)
def method_no_parens first_param,
                     second_param
  first_param
end
