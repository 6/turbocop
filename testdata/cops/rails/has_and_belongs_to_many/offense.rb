class User < ApplicationRecord
  has_and_belongs_to_many :roles
  ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/HasAndBelongsToMany: Use `has_many :through` instead of `has_and_belongs_to_many`.
end
