class User < ApplicationRecord
  attribute :tags, default: []
  ^^^^^^^^^ Rails/AttributeDefaultBlockValue: Pass a block to `default:` to avoid sharing mutable objects.
end

class Post < ApplicationRecord
  attribute :metadata, default: {}
  ^^^^^^^^^ Rails/AttributeDefaultBlockValue: Pass a block to `default:` to avoid sharing mutable objects.
end

class Order < ApplicationRecord
  attribute :confirmed_at, :datetime, default: Time.zone.now
  ^^^^^^^^^ Rails/AttributeDefaultBlockValue: Pass a block to `default:` to avoid sharing mutable objects.
end
