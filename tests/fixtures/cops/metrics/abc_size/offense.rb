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
^^^ Metrics/AbcSize: Assignment Branch Condition size for branchy_method is too high. [18.38/17]
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
^^^ Metrics/AbcSize: Assignment Branch Condition size for mixed_method is too high. [17.15/17]
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

# define_method blocks are treated as method definitions for ABC scoring
define_method(:complex_dm) do
^^^ Metrics/AbcSize: Assignment Branch Condition size for complex_dm is too high. [18.00/17]
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

# []= setter calls count as assignments and branches in RuboCop's ABC metric.
def indexed_assignment_heavy
^^^ Metrics/AbcSize: Assignment Branch Condition size for indexed_assignment_heavy is too high. [17.69/17]
  hash = {}
  hash[:a] = 1
  hash[:b] = 2
  hash[:c] = 3
  hash[:d] = 4
  hash[:e] = 5
  hash[:f] = 6
  hash[:g] = 7
  hash[:h] = 8
  hash[:i] = 9
  hash[:j] = 10
  hash[:k] = 11
  hash[:l] = 12
end

# Multi-assignment targets each count as an assignment in RuboCop.
# a, b, c, ... = x.split('|') => each target is an assignment.
# A=18 (targets), B=1 (split), C=0 => score = sqrt(324+1) = 18.03
def multi_write_method(data)
^^^ Metrics/AbcSize: Assignment Branch Condition size for multi_write_method is too high. [18.03/17]
  a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r = data.split("|")
end

# rescue => var counts as an assignment in RuboCop (lvasgn in Parser AST).
# A=17 (a..p + err), B=1 (foo), C=1 (rescue) => sqrt(289+1+1) = 17.06
def method_with_rescue_var
^^^ Metrics/AbcSize: Assignment Branch Condition size for method_with_rescue_var is too high. [17.06/17]
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
  begin
    foo
  rescue => err
  end
end

# Lambda literals (-> {}) count as a branch in RuboCop.
# In Parser AST, -> {} is (block (send nil :lambda) ...) and the :lambda send
# counts as B+1. In Prism, -> {} is LambdaNode with no CallNode.
# A=13, B=13 (lambda implicit calls), C=0 => sqrt(169+169) = 18.38
def method_with_many_lambdas
^^^ Metrics/AbcSize: Assignment Branch Condition size for method_with_many_lambdas is too high. [18.38/17]
  a = -> {}
  b = -> {}
  c = -> {}
  d = -> {}
  e = -> {}
  f = -> {}
  g = -> {}
  h = -> {}
  i = -> {}
  j = -> {}
  k = -> {}
  l = -> {}
  m = -> {}
end

# RuboCop's compound_assignment quirk: for shorthand assignments (||=, &&=, +=),
# if the value is a non-setter method call, it counts as an extra assignment.
# Each `x ||= fetch_val` in RuboCop produces: A+2 (lvasgn + compound_assignment),
# B+1 (fetch_val send), C+1 (or_asgn condition).
# 7 such lines: A=14, B=7, C=7 => sqrt(196+49+49) = sqrt(294) = 17.15
def method_with_or_assign_calls
^^^ Metrics/AbcSize: Assignment Branch Condition size for method_with_or_assign_calls is too high. [17.15/17]
  a ||= fetch_val
  b ||= fetch_val
  c ||= fetch_val
  d ||= fetch_val
  e ||= fetch_val
  f ||= fetch_val
  g ||= fetch_val
end

# Multi-write with CallTargetNode: `r.color, r.key = ...` are CallTargetNodes in Prism
# but regular :send nodes in Parser (counted as branches).
# A=14 (r, r_key/r_value/r_color, b, left=, @left, @right, right=, 3 call targets, @key/@value)
# B=13 (Object.new, r.key/r.value/r.color RHS, r.left, left=, r.right, right=, 3 CallTargetNode LHS, r.update_size, update_size)
# C=0
# score = sqrt(196+169) = 19.10
def multi_write_call_targets
^^^ Metrics/AbcSize: Assignment Branch Condition size for multi_write_call_targets is too high. [19.10/17]
  r = Object.new
  r_key, r_value, r_color = r.key, r.value, r.color
  b = r.left
  r.left = @left
  @left = r
  @right = r.right
  r.right = b
  r.color, r.key, r.value = :red, @key, @value
  @key, @value = r_key, r_value
  r.update_size
  update_size
end

# Interpolated regex =~ should count as B+1 (not skipped like literal regex).
# In Parser gem, /#{ interp }/ =~ expr is (send (regexp ...) :=~ expr) — counted as B+1.
# Only non-interpolated /literal/ =~ expr becomes match_with_lvasgn (NOT counted).
# A=17, B=1 (interpolated =~), C=0 => sqrt(289+1) = sqrt(290) = 17.03
def method_with_interpolated_regex_branch
^^^ Metrics/AbcSize: Assignment Branch Condition size for method_with_interpolated_regex_branch is too high. [17.03/17]
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
  q = /#{a}/ =~ "test"
end

# Nested rescue inside rescue body should count as separate C+1.
# The in_rescue_chain flag must not suppress nested begin...rescue...end.
# A=16, B=5 (foo,bar,baz,qux,quux), C=3 (if, outer rescue, nested rescue)
# => sqrt(256+25+9) = sqrt(290) = 17.03
# Without nested rescue counted: C=2 => sqrt(256+25+4) = sqrt(285) = 16.88 < 17
def method_with_nested_rescue_in_body
^^^ Metrics/AbcSize: Assignment Branch Condition size for method_with_nested_rescue_in_body is too high. [17.03/17]
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
  p = foo(bar(baz(qux(quux))))
  if a
    begin
      nil
    rescue
      begin
        nil
      rescue
        nil
      end
    end
  end
end
