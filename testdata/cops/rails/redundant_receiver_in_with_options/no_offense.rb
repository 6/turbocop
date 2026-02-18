class Account < ApplicationRecord
  with_options dependent: :destroy do
    has_many :customers
    has_many :products
    has_many :invoices
    has_many :expenses
  end
end

with_options options: false do |merger|
end
