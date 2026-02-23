foo.map { |x| [x, x * 2] }.to_h
    ^^^ Style/MapToHash: Pass a block to `to_h` instead of calling `map.to_h`.

foo.collect { |x, y| [x.to_s, y.to_i] }.to_h
    ^^^^^^^ Style/MapToHash: Pass a block to `to_h` instead of calling `collect.to_h`.

items.map { |(k, v)| [k, v * 2] }.to_h
      ^^^ Style/MapToHash: Pass a block to `to_h` instead of calling `map.to_h`.
