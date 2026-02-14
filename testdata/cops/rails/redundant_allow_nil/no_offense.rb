class User < ApplicationRecord
  validates :name, presence: true
  validates :email, allow_nil: true
  validates :phone, format: { with: /\A\d+\z/ }
  validates :age, numericality: true
end
