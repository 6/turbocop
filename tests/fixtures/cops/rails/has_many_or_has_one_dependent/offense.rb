class User < ApplicationRecord
  has_many :posts
  ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
  has_one :profile
  ^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
  has_many :comments
  ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
end

class Account < ApplicationRecord
  has_many :entries, class_name: 'Ledger'
  ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
  has_one :logo, class_name: 'Image'
  ^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
end

# Module concern with included block
module Taggable
  extend ActiveSupport::Concern

  included do
    has_many :tags
    ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
    has_one :primary_tag
    ^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
  end
end

# has_many with lambda scope but no dependent
class Article < ApplicationRecord
  has_many :items, -> { where(active: true) }
  ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
end

# through: nil should still flag
class Person < ApplicationRecord
  has_many :records, through: nil
  ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
  has_one :badge, through: nil
  ^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
end

# readonly? returning false should still flag
class Report < ActiveRecord::Base
  has_one :attachment
  ^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.

  def readonly?
    false
  end
end

# with_options block with through: nil should still flag
class Team < ApplicationRecord
  with_options through: nil do
    has_many :members
    ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
  end
end

# has_many on explicit receiver in module
module Trackable
  def self.included(base)
    base.has_many :events
         ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
  end
end

# has_many with double splat variable (unknown options)
class Widget < ApplicationRecord
  has_one :config, **opts
  ^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
end

# has_many with block AND extra args inside with_options (RuboCop doesn't
# recognize association extension blocks when extra args are present)
class KiwiUser < ApplicationRecord
  with_options dependent: :destroy do
    has_many :access_logs, class_name: "Kiwi::AccessLog" do
    ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
      def uniq_histories(max: 100)
      end
    end
  end
end

# has_many with explicit hash literal before lambda (not recognized as options)
class Contact < ActiveRecord::Base
  has_many :photos, {dependent: :destroy}, -> { order(created_at: :asc) }
  ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
end

# has_many inside intermediate block within with_options
# (with_options context does not propagate through non-with_options blocks)
class Vendor < ApplicationRecord
  with_options dependent: :destroy do
    ["orders", "products"].each do |model|
      has_many model.pluralize.to_sym
      ^^^^^^^^ Rails/HasManyOrHasOneDependent: Specify a `:dependent` option.
    end
  end
end
