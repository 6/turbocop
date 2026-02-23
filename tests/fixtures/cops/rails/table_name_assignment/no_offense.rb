class User < ActiveRecord::Base
  has_many :posts
end

class Post < ApplicationRecord
  belongs_to :user
end
