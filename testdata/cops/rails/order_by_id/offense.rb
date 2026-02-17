User.order(:id)
     ^^^^^ Rails/OrderById: Do not use the `id` column for ordering. Use a timestamp column to order chronologically.
Post.order(id: :asc)
     ^^^^^ Rails/OrderById: Do not use the `id` column for ordering. Use a timestamp column to order chronologically.
Comment.order(id: :desc)
        ^^^^^ Rails/OrderById: Do not use the `id` column for ordering. Use a timestamp column to order chronologically.
