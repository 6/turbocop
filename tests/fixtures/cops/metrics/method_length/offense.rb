def long_method
^^^ Metrics/MethodLength: Method has too many lines. [11/10]
  x = 1
  x = 2
  x = 3
  x = 4
  x = 5
  x = 6
  x = 7
  x = 8
  x = 9
  x = 10
  x = 11
end

def another_long_method
^^^ Metrics/MethodLength: Method has too many lines. [12/10]
  a = 1
  b = 2
  c = 3
  d = 4
  e = 5
  f = 6
  g = 7
  h = 8
  i = 9
  j = 10
  k = 11
  l = 12
end

def verbose_method(x)
^^^ Metrics/MethodLength: Method has too many lines. [14/10]
  if x
    a = 1
    b = 2
    c = 3
  else
    d = 4
    e = 5
    f = 6
  end
  g = 7
  h = 8
  i = 9
  j = 10
  k = 11
end

define_method(:dynamic_long) do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Metrics/MethodLength: Method has too many lines. [11/10]
  a = 1
  b = 2
  c = 3
  d = 4
  e = 5
  f = 6
  g = 7
  h = 8
  i = 9
  j = 10
  k = 11
end

define_method(:another_dynamic) { |x|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Metrics/MethodLength: Method has too many lines. [11/10]
  a = 1
  b = 2
  c = 3
  d = 4
  e = 5
  f = 6
  g = 7
  h = 8
  i = 9
  j = 10
  k = 11
}
