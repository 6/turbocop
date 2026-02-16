users.pluck(:name)
users.map { |u| u.name }
users.map { |u| u[:name].upcase }
users.select(:name)
users.collect(&:id)
# Receiver of [] is not the block param
ids.map { |id| accounts_map[id] }
rows.map { |row| row.data['domain'] }
keys.map { |key| key.split(':')[2] }
# Nested inside another block with a receiver — skip to prevent N+1 queries
# The inner map { |e| e['timestamp'] } is inside the outer responses.map block
responses.map { |r| r.map { |e| e[:timestamp] } }
