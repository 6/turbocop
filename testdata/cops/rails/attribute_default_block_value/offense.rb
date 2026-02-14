class User < ApplicationRecord
  attribute :tags, default: []
  ^^^^^^^^^ Rails/AttributeDefaultBlockValue: Pass a block to `default:` to avoid sharing mutable objects.
end
