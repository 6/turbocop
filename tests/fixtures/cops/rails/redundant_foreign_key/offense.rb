# belongs_to: default FK is {assoc_name}_id
belongs_to :user, foreign_key: :user_id
                  ^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.

belongs_to :author, foreign_key: :author_id
                    ^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.

belongs_to :category, foreign_key: "category_id"
                      ^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.

# belongs_to with class_name: FK is still based on assoc name, not class_name
belongs_to :post, class_name: 'SpecialPost', foreign_key: :post_id
                                             ^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.

# has_many: default FK is {model_name}_id
class Book
  has_many :chapters, foreign_key: :book_id
                      ^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
end

# has_one: default FK is {model_name}_id
class User
  has_one :profile, foreign_key: :user_id
                    ^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
end

# has_and_belongs_to_many: default FK is {model_name}_id
class Book
  has_and_belongs_to_many :authors, foreign_key: :book_id
                                    ^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
end

# has_many with :as option: default FK is {as_value}_id
class Book
  has_many :chapters, as: :publishable, foreign_key: :publishable_id
                                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
end

# has_one with :as option: default FK is {as_value}_id
class User
  has_one :avatar, as: :attachable, foreign_key: :attachable_id
                                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
end

# has_many with string FK value
class Book
  has_many :chapters, foreign_key: "book_id"
                      ^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
end

# CamelCase class name
class UserProfile
  has_many :settings, foreign_key: :user_profile_id
                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
end

# belongs_to with string association name
belongs_to "user", foreign_key: :user_id
                   ^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.

# Multiline belongs_to — offense on the foreign_key: line
belongs_to :reviewer,
           foreign_key: :reviewer_id
           ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.

# Multiline has_many — offense on the foreign_key: line
class Order
  has_many :items,
           foreign_key: :order_id
           ^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
end

# Multiline has_one with multiple options — offense on the foreign_key: line
class Account
  has_one :setting,
          class_name: 'AccountSetting',
          foreign_key: :account_id
          ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
end

# belongs_to with trailing block — RuboCop still flags these
belongs_to :layer_group, resource: GroupResource, writable: false, foreign_key: :layer_group_id do
                                                                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
  assign do |_groups, _layer_groups|
  end
end

# has_many inside ClassName.class_eval — RuboCop resolves the class from the receiver
Organization.class_eval do
  has_many :ai_models,
    class_name: "CustomAiModel",
    foreign_key: "organization_id",
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantForeignKey: Specifying the default value for `foreign_key` is redundant.
    limited_by_pricing_plans: { limit_key: :ai_models }
end
