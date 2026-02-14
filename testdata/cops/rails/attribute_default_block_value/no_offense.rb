class User < ApplicationRecord
  attribute :tags, default: -> { [] }
  attribute :active, default: true
  attribute :role, default: :member
end
