Post.where(user_id: User.select(:id))
Post.where(status: "active")
User.pluck(:id)
