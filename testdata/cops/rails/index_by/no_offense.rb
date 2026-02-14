users.index_by(&:id)
users.map { |u| u.name }.uniq
{}.to_h
users.each_with_object({}) { |u, h| h[u.id] = u.name }
users.group_by(&:role)
