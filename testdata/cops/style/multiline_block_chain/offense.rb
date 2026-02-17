Thread.list.select do |t|
  t.alive?
end.map do |t|
    ^^^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  t.object_id
end

items.select { |i|
  i.valid?
}.map { |i|
  ^^^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  i.name
}

foo.each do |x|
  x
end.count
    ^^^^^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
