User.ids
User.pluck(:name)
User.pluck(:id, :name)
User.select(:id)
User.where(active: true).ids
Post.where(user_id: User.pluck(:id))
Post.where(user_id: Comment.pluck(:id)).order(:created_at)
Post.rewhere(user_id: User.pluck(:id))
Post.where(user_id: User.pluck(:id)).not.order(:created_at)
