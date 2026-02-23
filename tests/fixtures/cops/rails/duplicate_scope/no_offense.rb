class Post < ApplicationRecord
  scope :published, -> { where(published: true) }
  scope :draft, -> { where(published: false) }
  scope :recent, -> { order(created_at: :desc) }
  scope :featured, -> { where(featured: true) }
end

# Same name, different body — NOT a duplicate expression
class Stance < ApplicationRecord
  scope :guests, -> { where(guest: true) }
  scope :guests, -> { where("inviter_id is not null") }
end
