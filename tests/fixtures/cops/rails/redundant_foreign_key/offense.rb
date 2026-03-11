# belongs_to: default FK is {assoc_name}_id
belongs_to :user, foreign_key: :user_id
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.

belongs_to :author, foreign_key: :author_id
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.

belongs_to :category, foreign_key: "category_id"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.

# belongs_to with class_name: FK is still based on assoc name, not class_name
belongs_to :post, class_name: 'SpecialPost', foreign_key: :post_id
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.

# has_many: default FK is {model_name}_id
class Book
  has_many :chapters, foreign_key: :book_id
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.
end

# has_one: default FK is {model_name}_id
class User
  has_one :profile, foreign_key: :user_id
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.
end

# has_and_belongs_to_many: default FK is {model_name}_id
class Book
  has_and_belongs_to_many :authors, foreign_key: :book_id
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.
end

# has_many with :as option: default FK is {as_value}_id
class Book
  has_many :chapters, as: :publishable, foreign_key: :publishable_id
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.
end

# has_one with :as option: default FK is {as_value}_id
class User
  has_one :avatar, as: :attachable, foreign_key: :attachable_id
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.
end

# has_many with string FK value
class Book
  has_many :chapters, foreign_key: "book_id"
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.
end

# CamelCase class name
class UserProfile
  has_many :settings, foreign_key: :user_profile_id
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.
end

# belongs_to with string association name
belongs_to "user", foreign_key: :user_id
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Redundant `foreign_key` -- it matches the default.
