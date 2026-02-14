def complex_method(a)
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for complex_method is too high. [8/7]
  if a == 1
    1
  end
  if a == 2
    2
  end
  if a == 3
    3
  end
  if a == 4
    4
  end
  if a == 5
    5
  end
  if a == 6
    6
  end
  if a == 7
    7
  end
end

def branchy_method(x)
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for branchy_method is too high. [9/7]
  if x > 0
    1
  end
  if x > 1
    2
  end
  if x > 2
    3
  end
  while x > 10
    x -= 1
  end
  until x < 0
    x += 1
  end
  if x > 3
    3
  end
  if x > 4
    4
  end
  if x > 5
    5
  end
end

def logical_method(a, b, c)
^^^ Metrics/CyclomaticComplexity: Cyclomatic complexity for logical_method is too high. [8/7]
  if a
    1
  end
  if b
    2
  end
  x = a && b
  y = b || c
  z = a && c
  w = a || b
  if c
    3
  end
end
