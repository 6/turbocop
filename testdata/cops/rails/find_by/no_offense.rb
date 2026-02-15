User.find_by(name: "foo")
User.where(name: "foo").last
User.where(name: "foo").to_a
User.find(1)
User.where(active: true).count
User.where(name: "foo").first
