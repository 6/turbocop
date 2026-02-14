class User < ActiveRecord::Base
  after_commit :do_first
  after_commit :do_second
  ^^^^^^^^^^^^^^^^^^^^^^^ Rails/AfterCommitOverride: Multiple `after_commit` callbacks may override each other.
end

class Post < ActiveRecord::Base
  after_create_commit :notify
  after_create_commit :log_event
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/AfterCommitOverride: Multiple `after_commit` callbacks may override each other.
end

class Order < ActiveRecord::Base
  after_commit :send_receipt
  after_commit :update_inventory
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/AfterCommitOverride: Multiple `after_commit` callbacks may override each other.
  after_commit :notify_warehouse
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/AfterCommitOverride: Multiple `after_commit` callbacks may override each other.
end
