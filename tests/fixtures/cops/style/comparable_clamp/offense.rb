if x < low
^^^^^^^^^^ Style/ComparableClamp: Use `clamp` instead of `if/elsif/else`.
  low
elsif high < x
  high
else
  x
end

if low > x
^^^^^^^^^^ Style/ComparableClamp: Use `clamp` instead of `if/elsif/else`.
  low
elsif high < x
  high
else
  x
end

if x < low
^^^^^^^^^^ Style/ComparableClamp: Use `clamp` instead of `if/elsif/else`.
  low
elsif x > high
  high
else
  x
end

2 * [[(sign_in_count - 2), 0].max, 3].min
    ^ Style/ComparableClamp: Use `Comparable#clamp` instead.

[[value.to_f, 255].min, 0].max
^ Style/ComparableClamp: Use `Comparable#clamp` instead.

prices << self.price = [[@price + @delta_sign*delta, Stock.price_min].max, Stock.price_max].min
                       ^ Style/ComparableClamp: Use `Comparable#clamp` instead.

y3 = [[y3 + (rand*24 - 12), 0].max, 700].min
     ^ Style/ComparableClamp: Use `Comparable#clamp` instead.

value = [[1, value.to_i].max, 3].min
        ^ Style/ComparableClamp: Use `Comparable#clamp` instead.

start_line = [ [ start_line, 0 ].max, total ].min
             ^ Style/ComparableClamp: Use `Comparable#clamp` instead.

end_line = [ [ end_line, start_line ].max, total ].min
           ^ Style/ComparableClamp: Use `Comparable#clamp` instead.

middle_stars = "*" * [ 0, [ 10, token.length - 6 ].min ].max
                     ^ Style/ComparableClamp: Use `Comparable#clamp` instead.
