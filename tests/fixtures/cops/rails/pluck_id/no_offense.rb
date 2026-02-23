User.ids
User.pluck(:name)
User.pluck(:id, :name)
User.select(:id)
User.where(active: true).ids
