class User < ApplicationRecord
  validates :name, presence: true
  validates :email, allow_nil: true
end
