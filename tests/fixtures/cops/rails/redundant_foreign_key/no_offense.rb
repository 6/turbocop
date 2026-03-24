# belongs_to without foreign_key
belongs_to :user
belongs_to :author

# belongs_to with non-default foreign_key
belongs_to :user, foreign_key: :author_id
belongs_to :author, foreign_key: :user_id

# has_many without foreign_key
class Post
  has_many :comments
  has_many :comments, dependent: :destroy
end

# has_many with non-default foreign_key
class Post
  has_many :comments, foreign_key: :author_id
end

# has_one with non-default foreign_key
class User
  has_one :profile, foreign_key: :account_id
end

# has_many not inside a class (can't determine model name)
has_many :chapters, foreign_key: :book_id

# has_one not inside a class
has_one :profile, foreign_key: :user_id

# has_many inside a block (not a class)
class_methods do
  has_many :chapters, foreign_key: :book_id
end

# has_many with :as option and non-default FK
class Book
  has_many :chapters, as: :publishable, foreign_key: :book_id
end

# has_and_belongs_to_many not inside a class
has_and_belongs_to_many :authors, foreign_key: :book_id

# has_many/has_one with scope lambda — RuboCop skips these (only checks 2-arg calls)
class Hardware
  has_many :hard_disks, -> { where.not(device_type: 'floppy').order(:location) }, class_name: "Disk", foreign_key: :hardware_id
  has_many :floppies, -> { where(device_type: 'floppy') }, class_name: "Disk", foreign_key: :hardware_id
end

class User
  has_one :recent_post, -> { order(created_at: :desc) }, class_name: "Post", foreign_key: :user_id
end

# has_many with trailing block — RuboCop skips these
class Post
  has_many :comments, foreign_key: :post_id do
    def custom
    end
  end
end

# has_many inside with_options block — RuboCop's parent_module_name returns nil
# because the with_options block is an ancestor of the send node
class Game
  with_options dependent: :destroy do
    has_many :challenges, inverse_of: :game, foreign_key: 'game_id'
  end
end

# has_many inside a class that is itself inside a block (e.g., RSpec test)
describe "test" do
  class Paz < ActiveRecord::Base
    has_many :other_zaps, :class_name => "Zap", :foreign_key => :paz_id
  end
end

# has_many inside any non-class_eval block, even when redundant
class Usergroup
  concern do
    has_many :cached_usergroup_members, :foreign_key => 'usergroup_id'
  end
end
