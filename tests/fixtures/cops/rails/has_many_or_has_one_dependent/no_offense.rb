class User < ApplicationRecord
  has_many :posts, dependent: :destroy
  has_one :profile, dependent: :nullify
  has_many :comments, through: :posts
  has_many :likes, dependent: :delete_all
  has_one :address, dependent: :destroy
end
