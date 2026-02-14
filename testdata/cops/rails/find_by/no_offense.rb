User.find_by(name: "foo")
User.where(name: "foo").last
User.where(name: "foo").to_a