alive_threads = Thread.list.select do |t|
  t.alive?
end
alive_threads.map do |t|
  t.object_id
end

items.select { |i| i.valid? }.map { |i| i.name }

foo.each { |x| x }.count

# Multiline block with method chain (no block on .count) — not a block chain
foo.each do |x|
  x
end.count

# expect block with .not_to — not a block chain
expect do
  Fabricate(:problem)
end.not_to change(Comment, :count)

# expect block with .to — not a block chain
expect do
  Fabricate(:problem)
end.to change(Comment, :count).by(3)

# Single-line block chains — no offense
a { b }.c { d }
w do x end.y do z end

# Chain of calls followed by a multi-line block — not a block chain
a1.a2.a3 do
  b
end

# First block is single-line, second is multiline — no offense
Thread.list.find_all { |t| t.alive? }.map { |t|
  t.object_id
}
