User.where(age: 18..)
User.where(name: "foo")
User.where("name LIKE ?", "%foo%")