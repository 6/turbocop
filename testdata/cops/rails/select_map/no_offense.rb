users.filter_map { |u| u.name if u.active? }
users.select(:name)
users.map { |u| u.name }
users.pluck(:name)
users.where(active: true).map(&:name)
