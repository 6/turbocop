User.pluck(:id)
     ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

Post.where(active: true).pluck(:id)
                         ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

Comment.pluck(:id)
        ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

User&.pluck(:id)
      ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

def self.user_ids
  pluck(primary_key)
  ^^^^^^^^^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(primary_key)`.
end

Post.pluck(:id).where(id: 1..10)
     ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.
