users.pluck(:name)
users.map { |u| u.name }
users.map { |u| u[:name].upcase }
