# Simple block chain with do..end
Thread.list.select do |t|
  t.alive?
end.map do |t|
^^^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  t.object_id
end

# Simple block chain with braces
items.select { |i|
  i.valid?
}.map { |i|
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  i.name
}

# Another simple block chain
foo.each do |x|
  x
end.map do |y|
^^^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  y.to_s
end

# Intermediate non-block calls between blocks
a do
  b
end.c1.c2 do
^^^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  d
end

# Safe navigation operator
a do
  b
end&.c do
^^^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  d
end

# Chain of three blocks — two offenses
a do
  b
end.c do
^^^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  d
end.e do
^^^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  f
end

# Second block is single-line but first is multiline
Thread.list.find_all { |t|
  t.alive?
}.map { |thread| thread.object_id }
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.

# Dot on next line after end (multiline chain)
items.select do |i|
  i.valid?
end
^^^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  .map do |i|
  i.name
end
