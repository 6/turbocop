class User < ApplicationRecord
  has_many :posts, dependent: :destroy
  has_one :profile, dependent: :nullify
  has_many :comments, through: :posts
end
