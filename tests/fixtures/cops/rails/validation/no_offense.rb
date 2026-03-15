class User < ApplicationRecord
  validates :name, presence: true
  validates :email, uniqueness: true
  validates :age, numericality: { greater_than: 0 }
  validates :role, inclusion: { in: %w[admin user] }
end

# No arguments — RuboCop skips these
validates_numericality_of
validates_presence_of
