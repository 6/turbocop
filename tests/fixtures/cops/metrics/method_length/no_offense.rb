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

# RuboCop counts a method body that is only a heredoc expression as one line.
# Even with large heredoc content, this should not trigger MethodLength.
def heredoc_only_method
  <<~SQL
    line1
    line2
    line3
    line4
    line5
    line6
    line7
    line8
    line9
    line10
    line11
    line12
    line13
    line14
    line15
    line16
    line17
    line18
    line19
    line20
    line21
    line22
    line23
    line24
    line25
    line26
    line27
    line28
    line29
    line30
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

# Receiver-qualified define_method with short body (no offense)
builder.define_method(:short_generated) do
  a = 1
  b = 2
  c = 3
end

# Endless method with short multiline body (no offense)
def compact_settings = {
  one: 1,
  two: 2,
  three: 3
}
