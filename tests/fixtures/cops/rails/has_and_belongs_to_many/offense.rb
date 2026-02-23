class User < ApplicationRecord
  has_and_belongs_to_many :roles
  ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/HasAndBelongsToMany: Use `has_many :through` instead of `has_and_belongs_to_many`.
end

class Post < ApplicationRecord
  has_and_belongs_to_many :tags
  ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/HasAndBelongsToMany: Use `has_many :through` instead of `has_and_belongs_to_many`.
end

class Student < ApplicationRecord
  has_and_belongs_to_many :courses
  ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/HasAndBelongsToMany: Use `has_many :through` instead of `has_and_belongs_to_many`.
end
