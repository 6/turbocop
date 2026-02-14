User.all.find_each { |u| u.save }
[1, 2, 3].each { |x| puts x }
users.each { |u| u.save }