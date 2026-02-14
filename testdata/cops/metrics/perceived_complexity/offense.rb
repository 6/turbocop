def complex_method(a)
^^^ Metrics/PerceivedComplexity: Perceived complexity for complex_method is too high. [9/8]
  if a == 1
    1
  else
    0
  end
  if a == 2
    2
  else
    0
  end
  if a == 3
    3
  else
    0
  end
  if a == 4
    4
  else
    0
  end
end

def looping_method(x)
^^^ Metrics/PerceivedComplexity: Perceived complexity for looping_method is too high. [10/8]
  while x > 0
    x -= 1
  end
  until x < 10
    x += 1
  end
  for i in [1, 2, 3]
    puts i
  end
  if x == 1
    1
  else
    0
  end
  if x == 2
    2
  else
    0
  end
  if x == 3
    3
  else
    0
  end
end

def case_method(x)
^^^ Metrics/PerceivedComplexity: Perceived complexity for case_method is too high. [10/8]
  case x
  when 1
    :one
  when 2
    :two
  when 3
    :three
  when 4
    :four
  when 5
    :five
  when 6
    :six
  when 7
    :seven
  when 8
    :eight
  when 9
    :nine
  end
end
