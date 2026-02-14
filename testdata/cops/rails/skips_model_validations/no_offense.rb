user.update(name: "new")
user.save
user.save!
User.create(name: "new")
User.find_or_create_by(name: "test")
