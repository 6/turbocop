User.where(active: true)
User.order(:name)
User.all
User.all.each { |u| u.save }