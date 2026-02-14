class User < ApplicationRecord
  has_many :posts
  has_many :posts, dependent: :destroy
  ^^^^^^^^ Rails/DuplicateAssociation: Duplicate association `posts` detected.
end
