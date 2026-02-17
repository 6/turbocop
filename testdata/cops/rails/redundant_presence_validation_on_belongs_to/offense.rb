class Post < ApplicationRecord
  belongs_to :user
  validates :user, presence: true
  ^^^^^^^^^ Rails/RedundantPresenceValidationOnBelongsTo: Remove explicit `presence` validation for `user` `belongs_to` association (validated by default since Rails 5).
end

class Comment < ApplicationRecord
  belongs_to :post
  validates :post, presence: true
  ^^^^^^^^^ Rails/RedundantPresenceValidationOnBelongsTo: Remove explicit `presence` validation for `post` `belongs_to` association (validated by default since Rails 5).
end

class Order < ApplicationRecord
  belongs_to :customer
  validates :customer, presence: true
  ^^^^^^^^^ Rails/RedundantPresenceValidationOnBelongsTo: Remove explicit `presence` validation for `customer` `belongs_to` association (validated by default since Rails 5).
end

class Err
  belongs_to :problem
  validates :problem_id, :fingerprint, presence: true
  ^^^^^^^^^ Rails/RedundantPresenceValidationOnBelongsTo: Remove explicit `presence` validation for `problem_id` `belongs_to` association (validated by default since Rails 5).
end
