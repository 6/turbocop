class User < ApplicationRecord
  validates :name, presence: true
  validates :email, allow_nil: true
  validates :phone, allow_blank: true
  validates :age, numericality: true
  validates :x, length: { is: 5 }, allow_nil: true, allow_blank: false
  validates :y, length: { is: 5 }, allow_blank: true
  validates :z, length: { is: 5 }, allow_nil: true
end
