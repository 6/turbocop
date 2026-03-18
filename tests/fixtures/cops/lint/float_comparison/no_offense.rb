x == 0.0
x != 0.0
x == 1
x == nil
x == y
(x - 0.1).abs < Float::EPSILON
x.to_f == 0
x.to_f == 0.0
x.to_f == nil

case value
when 0.0
  foo
end

case value
when 1
  foo
when 'string'
  bar
end

x == 1.1.ceil
x == 1.1.floor
x == 1 + 2
x == (1)
# Parenthesized float expressions - RuboCop does not unwrap parens
x == (0.1)
(-0.0).next_float == (0.0).next_float
(0.0).prev_float == (-0.0).prev_float
