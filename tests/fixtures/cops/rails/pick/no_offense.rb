User.pick(:name)
User.pluck(:name).last
User.pluck(:name)
User.where(active: true).pick(:email)
User.first
