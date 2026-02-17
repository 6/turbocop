def complex_method
^^^ Metrics/AbcSize: Assignment Branch Condition size for complex_method is too high. [18.00/17]
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
  m = 13
  n = 14
  o = 15
  p = 16
  q = 17
  r = 18
end

def branchy_method(x)
^^^ Metrics/AbcSize: Assignment Branch Condition size for branchy_method is too high. [19.10/17]
  a = x.foo
  b = x.bar
  c = x.baz
  d = x.qux
  e = x.quux
  f = x.corge
  g = x.grault
  h = x.garply
  i = x.waldo
  j = x.fred
  k = x.plugh
  l = x.xyzzy
  m = x.thud
end

def mixed_method(x)
^^^ Metrics/AbcSize: Assignment Branch Condition size for mixed_method is too high. [17.92/17]
  a = x.foo
  b = x.bar
  c = x.baz
  d = x.qux
  e = x.quux
  f = x.corge
  g = x.grault
  h = x.garply
  i = x.waldo
  j = x.fred
  k = x.plugh
  if a
    l = 1
  end
  if b
    m = 1
  end
end
