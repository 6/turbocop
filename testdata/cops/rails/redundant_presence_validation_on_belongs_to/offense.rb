class Post < ApplicationRecord
  belongs_to :user
  validates :user, presence: true
  ^^^^^^^^^ Rails/RedundantPresenceValidationOnBelongsTo: Remove explicit `presence` validation for `user` `belongs_to` association (validated by default since Rails 5).
end
