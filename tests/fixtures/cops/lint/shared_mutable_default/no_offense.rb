Hash.new([].freeze)
Hash.new({}.freeze)
Hash.new { |h, k| h[k] = [] }
Hash.new(0)
Hash.new('default')

# Qualified constant paths are not flagged — only bare Hash
Concurrent::Hash.new(Concurrent::Array.new)
MyModule::Hash.new([])

# capacity keyword argument is not a mutable default
Hash.new(capacity: 5)

# Array.new/Hash.new with arguments are not bare mutable constructors
Hash.new(Hash.new(0))
Hash.new(Array.new(5))
