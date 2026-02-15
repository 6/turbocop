class User < ApplicationRecord
  validates :x, length: { is: 5 }, allow_nil: true, allow_blank: true
                                   ^^^^^^^^^^^^^^^ Rails/RedundantAllowNil: `allow_nil` is redundant when `allow_blank` has the same value.
end

class Post < ApplicationRecord
  validates :x, length: { is: 5 }, allow_nil: false, allow_blank: false
                                   ^^^^^^^^^^^^^^^^ Rails/RedundantAllowNil: `allow_nil` is redundant when `allow_blank` has the same value.
end

class Comment < ApplicationRecord
  validates :x, length: { is: 5 }, allow_nil: false, allow_blank: true
                                   ^^^^^^^^^^^^^^^^ Rails/RedundantAllowNil: `allow_nil: false` is redundant when `allow_blank` is true.
end
