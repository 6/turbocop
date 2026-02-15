users.index_by(&:id)
users.map { |u| u.name }.uniq
{}.to_h
users.each_with_object({}) { |u, h| h[u.id] = u.name }
users.group_by(&:role)
# Block body is not [key, element] pattern
users.map { |u| [u.id, u.name] }.to_h
items.map { |i| [i.key, transform(i)] }.to_h
data.map { |d| { d.id => d } }.to_h
