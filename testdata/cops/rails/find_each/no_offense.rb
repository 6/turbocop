User.all.find_each { |u| u.save }
[1, 2, 3].each { |x| puts x }
users.each { |u| u.save }
User.find_each { |u| u.update(active: true) }
records.map { |r| r.name }
