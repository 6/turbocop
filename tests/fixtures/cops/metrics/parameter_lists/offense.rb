def too_many_params(a, b, c, d, e, f)
^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [6/5]
  a + b + c + d + e + f
end

def another_long(a, b, c, d, e, f, g)
^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [7/5]
  [a, b, c, d, e, f, g]
end

def with_keywords(a, b, c, d, e, f:)
^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [6/5]
  a
end
