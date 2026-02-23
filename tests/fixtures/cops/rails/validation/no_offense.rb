class User < ApplicationRecord
  validates :name, presence: true
  validates :email, uniqueness: true
  validates :age, numericality: { greater_than: 0 }
  validates :role, inclusion: { in: %w[admin user] }
end
