users.index_by(&:id)
users.map { |u| u.name }.uniq
{}.to_h
