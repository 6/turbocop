users.pluck(:name)
users.map { |u| u.name }
users.map { |u| u[:name].upcase }
users.select(:name)
users.collect(&:id)
# Receiver of [] is not the block param
ids.map { |id| accounts_map[id] }
rows.map { |row| row.data['domain'] }
keys.map { |key| key.split(':')[2] }
# Nested inside a block with receiver — skip to prevent N+1 queries
responses.map { |r| r.map { |e| e[:timestamp] } }
# Block with receiver wrapping map — nearest ancestor has receiver, skip
5.times do
  users.map { |u| u[:name] }
end
# Key is a regexp — skip
items.map { |item| item[/pattern/] }
# Key references the block argument — not a simple pluck
records.map { |r| r[r] }
# Key uses block arg in a nested expression
entries.map { |e| e[transform(e)] }
# Multiple arguments to [] — not a simple key lookup
items.map { |item| item[1, 2] }
