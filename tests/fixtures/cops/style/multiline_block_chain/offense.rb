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

# Descendant call inside [] receiver before block chaining
Hash[items.map do |item|
  item
end.compact].tap do |values|
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  values
end

# If/else result compacted before block chaining
(if hosts.nil?
  []
else
  hosts.map do |host|
    host
end.compact
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
end).each do |host|
  host
end

# Parenthesized operator receiver before block chaining
(Dir.entries(directory).select do |fp|
  fp.start_with?(filename)
end - (input[-1] == '.' ? [] : ['.', '..'])).map do |fp|
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  fp
end

# Parenthesized compact call before block chaining
(if filters.nil?
  items
else
  items.select do |item|
    item
  end.compact
  ^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
end).map do |item|
  item
end

# Block-pass chain inside hash literal before block chaining
{
  product: YAML.load_file(path).select do |model|
    model
end.map(&:classify).map(&:constantize) + [Tracking],
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
}.each do |name, models|
  name
end

# Binary + over a multiline brace block before block chaining
(
  zones.collect { |zone|
    [zone.name, zone]
} +
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  [
    ['UTC', 'UTC'],
  ]
).each do |zone|
  zone
end

# Reduced result with extra receiver chain before block chaining
(traverse_files do |path|
  scan_file(path)
end.reduce(:+) || []).group_by(&:first).map do |key, occurrences|
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  [key, occurrences]
end

# Parenthesized subtract/flatten/uniq chain before block chaining
((synsets.collect do |synset|
  synset.values
end - [word]).flatten).uniq.map do |token|
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  token
end

# Lambda receiver before block chaining
-> do
  raise "new error"
end.should raise_error(RuntimeError, "new error") do |error|
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  error
end

# Descendant join call inside arguments before block chaining
String.normalize_path(segments.map do |segment|
  segment
end.join("/")).tap do |matcher|
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  matcher
end

# Block chain inside lambda argument before outer block chaining
definable builder: -> {
  parts.map do |segment|
    segment
end.join("/")
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
} do
  body
end

# Descendant compact call inside constructor arguments before block chaining
Query::Solutions(left.map do |first|
  right.map do |second|
    second
  end
end.flatten.compact).map do |solution|
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
  solution
end

# Descendant intersperse call inside array literal before block chaining
[
  *drawable_items.flat_map.with_index do |item, i|
    render(item, i)
end.intersperse(nil),
^ Style/MultilineBlockChain: Avoid multi-line chains of blocks.
].reduce(nil) do |acc, item|
  item
end
