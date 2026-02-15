class User < ApplicationRecord
  attribute :tags, default: -> { [] }
  attribute :active, default: true
  attribute :role, default: :member
  attribute :login_count, :integer, default: 0
  attribute :notes, default: "pending"
  attribute :status, :string, default: "active"
end
