Hash.new([].freeze)
Hash.new({}.freeze)
Hash.new { |h, k| h[k] = [] }
Hash.new(0)
Hash.new('default')
