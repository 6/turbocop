User.where.not(status: "active")
User.where(status: "active")
User.where("name = ?", "foo")
