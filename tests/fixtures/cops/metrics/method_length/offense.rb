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

# Heredoc content counts when wrapped in another expression (assignment).
def heredoc_assignment_method
^^^ Metrics/MethodLength: Method has too many lines. [12/10]
  query = <<~SQL
    SELECT *
    FROM users
    WHERE active = true
    AND created_at > '2024-01-01'
    ORDER BY name ASC
    LIMIT 100
    OFFSET 0
    -- long query
    -- more comments
    -- even more
  SQL
end

def interpolated_heredoc_method
^^^ Metrics/MethodLength: Method has too many lines. [11/10]
  <<~TEXT
    line1
    line2
    line3
    line4
    line5
    line6
    line7
    line8
    #{dynamic_value}
  TEXT
end

# Receiver-qualified define_method should also be checked.
builder.define_method(:generated_method) do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Metrics/MethodLength: Method has too many lines. [11/10]
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

# Endless methods with multiline bodies should be checked.
def settings = {
^^^ Metrics/MethodLength: Method has too many lines. [13/10]
  one: 1,
  two: 2,
  three: 3,
  four: 4,
  five: 5,
  six: 6,
  seven: 7,
  eight: 8,
  nine: 9,
  ten: 10,
  eleven: 11
}
