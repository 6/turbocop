Hash.new([].freeze)
Hash.new({}.freeze)
Hash.new { |h, k| h[k] = [] }
Hash.new(0)
Hash.new('default')

# Qualified constant paths are not flagged — only bare Hash
Concurrent::Hash.new(Concurrent::Array.new)
MyModule::Hash.new([])
