foo.map { |x| [x, x * 2] }.to_set
    ^^^ Style/MapToSet: Pass a block to `to_set` instead of calling `map.to_set`.

foo.collect { |x, y| [x.to_s, y.to_i] }.to_set
    ^^^^^^^ Style/MapToSet: Pass a block to `to_set` instead of calling `collect.to_set`.

items.map { |x| x.to_s }.to_set
      ^^^ Style/MapToSet: Pass a block to `to_set` instead of calling `map.to_set`.
