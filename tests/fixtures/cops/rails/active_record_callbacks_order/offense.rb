class User < ApplicationRecord
  after_save :do_something
  before_save :prepare
  ^^^^^^^^^^^ Rails/ActiveRecordCallbacksOrder: `before_save` is supposed to appear before `after_save`.
end

class Post < ApplicationRecord
  after_commit :notify
  before_validation :normalize
  ^^^^^^^^^^^^^^^^^ Rails/ActiveRecordCallbacksOrder: `before_validation` is supposed to appear before `after_commit`.
end

class Order < ApplicationRecord
  after_destroy :cleanup
  before_destroy :check_status
  ^^^^^^^^^^^^^^ Rails/ActiveRecordCallbacksOrder: `before_destroy` is supposed to appear before `after_destroy`.
end
