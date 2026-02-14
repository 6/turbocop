class User < ApplicationRecord
  after_save :do_something
  before_save :prepare
  ^^^^^^^^^^^ Rails/ActiveRecordCallbacksOrder: Callback `before_save` should appear before `after_save`.
end
