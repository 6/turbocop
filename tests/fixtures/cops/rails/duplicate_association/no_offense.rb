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

# has_and_belongs_to_many without duplicates
class Category < ApplicationRecord
  has_and_belongs_to_many :posts
  has_and_belongs_to_many :tags
end

# belongs_to with same class_name is NOT flagged
class Order < ApplicationRecord
  belongs_to :foos, class_name: 'Foo'
  belongs_to :bars, class_name: 'Foo'
end

# class_name with extra options is NOT flagged
class Report < ApplicationRecord
  has_many :foos, if: :condition, class_name: 'Foo'
  has_many :bars, if: :some_condition, class_name: 'Foo'
  has_one :baz, -> { condition }, class_name: 'Bar'
  has_one :qux, -> { some_condition }, class_name: 'Bar'
end

# In RuboCop, duplicate associations inside a conditional are ignored when the
# class body has sibling statements. Parser represents that class body as a
# multi-statement `begin`, and `class_send_nodes` does not descend into the
# nested `if`.
class CompatModel < ActiveRecord::Base
  if ActiveRecord.version >= Gem::Version.new('5.0')
    belongs_to :person, optional: true
  else
    belongs_to :person
  end
  has_many :orders
end

# Namespaced ApplicationRecord subclasses are not treated as Active Record
# models by RuboCop's ActiveRecordHelper.
class CommitStatus < Ci::ApplicationRecord
  belongs_to :ci_stage
  belongs_to :ci_stage
end

# class_name duplicates are ignored when one association uses an extension block.
class Author < ActiveRecord::Base
  has_many :posts_containing_the_letter_a, class_name: 'Post'
  has_many :posts_with_extension, class_name: 'Post' do
    def testing_proxy_owner
      proxy_owner
    end
  end
end
