%w(foo bar baz) * ","
                ^ Style/ArrayJoin: Favor `Array#join` over `Array#*`.

[1, 2, 3] * "-"
          ^ Style/ArrayJoin: Favor `Array#join` over `Array#*`.

['a', 'b'] * " "
           ^ Style/ArrayJoin: Favor `Array#join` over `Array#*`.
