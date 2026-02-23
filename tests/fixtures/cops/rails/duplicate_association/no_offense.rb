class User < ApplicationRecord
  has_many :posts, dependent: :destroy
  has_one :profile, dependent: :destroy
  belongs_to :company
end

# Serializer classes should not be flagged
class CollectionSerializer < ActivityPub::Serializer
  has_many :items, key: :items, if: -> { condition_a }
  has_many :items, key: :ordered_items, if: -> { condition_b }
end
