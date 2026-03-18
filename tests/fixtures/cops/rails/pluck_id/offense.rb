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

current_user.events.pluck(:id)
                    ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

e.users.pluck(:id)
        ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

record.items.where(active: true).pluck(:id)
                                 ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

# pluck inside array concatenation inside where — vendor flags this (first call ancestor is `+`)
Post.where(story_id: [@record.id] + @record.children.pluck(:id))
                                                     ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

# pluck inside array union inside where
Enterprise.where(id: relatives.pluck(:id) | [id])
                               ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

# pluck inside arel .in() inside where
Model.where(items.arel_table[:group_id].in(groups.pluck(:id)))
                                                  ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

# pluck inside map block inside where
User.where(id: events.map { |e| e.users.pluck(:id) }.flatten)
                                        ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.

# pluck inside array append inside where.not
Post.where.not(id: [owner.id] + mentions.pluck(:id))
                                         ^^^^^^^^^^ Rails/PluckId: Use `ids` instead of `pluck(:id)`.
