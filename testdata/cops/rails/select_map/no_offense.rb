users.filter_map { |u| u.name if u.active? }
users.select(:name)
users.map { |u| u.name }
users.pluck(:name)
users.where(active: true).map(&:name)
users.select { |u| u.active? }.map { |u| u.name }
Model.select(:name).map(&:email)
queue.select { |s| s.klass == name }.map(&:delete)
