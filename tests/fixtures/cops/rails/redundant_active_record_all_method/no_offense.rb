User.where(active: true)
User.order(:name)
User.all
User.all.each { |u| u.save }
User.find(1)
all.select { |role| role.can?(:manage) }
User.all.select(&:active?)
