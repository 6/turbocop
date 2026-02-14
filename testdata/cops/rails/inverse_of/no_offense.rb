class User < ApplicationRecord
  has_many :posts
  has_many :posts, foreign_key: :author_id, inverse_of: :author
  has_one :profile, as: :profilable, inverse_of: :user
  belongs_to :company
end
