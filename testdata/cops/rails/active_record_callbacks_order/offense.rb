class User < ApplicationRecord
  after_save :do_something
  before_save :prepare
  ^^^^^^^^^^^ Rails/ActiveRecordCallbacksOrder: Callback `before_save` should appear before `after_save`.
end

class Post < ApplicationRecord
  after_commit :notify
  before_validation :normalize
  ^^^^^^^^^^^^^^^^^ Rails/ActiveRecordCallbacksOrder: Callback `before_validation` should appear before `after_commit`.
end

class Order < ApplicationRecord
  after_destroy :cleanup
  before_destroy :check_status
  ^^^^^^^^^^^^^^ Rails/ActiveRecordCallbacksOrder: Callback `before_destroy` should appear before `after_destroy`.
end
