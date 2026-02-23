class User < ApplicationRecord
  has_many :user_roles
  has_many :roles, through: :user_roles
end

class Post < ApplicationRecord
  has_many :post_tags
  has_many :tags, through: :post_tags
end
