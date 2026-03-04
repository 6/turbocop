def short_method
  x = 1
  x = 2
  x = 3
end

def ten_lines
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
end

def empty_method
end

def one_liner
  42
end

def with_branch
  if true
    1
  else
    2
  end
end

# Heredoc content lines count toward method length (RuboCop's
# CodeLengthCalculator includes them via source_from_node_with_heredoc).
# This heredoc has 8 content lines => 10 total body lines (at Max:10).
def heredoc_method
  <<~SQL
    SELECT *
    FROM users
    WHERE active = true
    AND created_at > '2024-01-01'
    ORDER BY name ASC
    LIMIT 100
    OFFSET 0
  SQL
end

# Multiline params should not count toward body length.
# RuboCop counts only body.source lines, not parameter lines.
def initialize(
  param1: nil,
  param2: nil,
  param3: nil,
  param4: nil,
  param5: nil,
  param6: nil,
  param7: nil,
  param8: nil,
  param9: nil,
  param10: nil
)
  a = param1
  b = param2
  c = param3
end

# define_method with short body (no offense)
define_method(:short_dynamic) do
  a = 1
  b = 2
  c = 3
end

# define_method at exactly Max lines
define_method(:ten_dynamic) do
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
end

# define_method with brace block
define_method(:brace_dynamic) { |x|
  a = 1
  b = 2
}

# define_method with string name
define_method("string_name") do
  a = 1
end
