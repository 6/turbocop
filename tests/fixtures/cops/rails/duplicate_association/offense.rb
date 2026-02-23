class User < ApplicationRecord
  has_many :posts
  has_many :posts, dependent: :destroy
  ^^^^^^^^ Rails/DuplicateAssociation: Duplicate association `posts` detected.
end

class Post < ApplicationRecord
  belongs_to :author
  belongs_to :author, optional: true
  ^^^^^^^^^^ Rails/DuplicateAssociation: Duplicate association `author` detected.
end

class Company < ApplicationRecord
  has_one :address
  has_one :address, dependent: :destroy
  ^^^^^^^ Rails/DuplicateAssociation: Duplicate association `address` detected.
end
