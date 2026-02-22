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

def many_ifs_method(x)
^^^ Metrics/PerceivedComplexity: Perceived complexity for many_ifs_method is too high. [9/8]
  if x == 1
    1
  end
  if x == 2
    2
  end
  if x == 3
    3
  end
  if x == 4
    4
  end
  if x == 5
    5
  end
  if x == 6
    6
  end
  if x == 7
    7
  end
  if x == 8
    8
  end
end

def iterating_and_index_or(values)
^^^ Metrics/PerceivedComplexity: Perceived complexity for iterating_and_index_or is too high. [10/8]
  values[:a] ||= 1
  values[:b] ||= 2
  values[:c] ||= 3
  values[:d] ||= 4
  values[:e] ||= 5
  values[:f] ||= 6
  values.delete_if { |v| v.nil? }
  values.reject! { |_k, v| v == false }
  values.map! { |v| v.to_s }
  values
end
