class User < ApplicationRecord
  has_many :posts
  ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
  has_one :profile
  ^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
  has_many :comments
  ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
end
