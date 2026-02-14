class User < ApplicationRecord
  attribute :tags, default: []
  ^^^^^^^^^ Rails/AttributeDefaultBlockValue: Pass a block to `default:` to avoid sharing mutable objects.
end

class Post < ApplicationRecord
  attribute :metadata, default: {}
  ^^^^^^^^^ Rails/AttributeDefaultBlockValue: Pass a block to `default:` to avoid sharing mutable objects.
end

class Order < ApplicationRecord
  attribute :notes, default: "pending"
  ^^^^^^^^^ Rails/AttributeDefaultBlockValue: Pass a block to `default:` to avoid sharing mutable objects.
end
