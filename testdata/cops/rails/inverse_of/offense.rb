class User < ApplicationRecord
  has_many :posts, foreign_key: :author_id
  ^^^^^^^^ Rails/InverseOf: Specify an `:inverse_of` option.
  has_one :profile, as: :profilable
  ^^^^^^^ Rails/InverseOf: Specify an `:inverse_of` option.
end
