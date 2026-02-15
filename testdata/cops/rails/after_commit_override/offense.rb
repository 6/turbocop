class User < ActiveRecord::Base
  after_create_commit :log_action
  after_update_commit :log_action
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/AfterCommitOverride: There can only be one `after_*_commit :log_action` hook defined for a model.
end

class Post < ActiveRecord::Base
  after_commit :notify, on: :create
  after_commit :notify, on: :destroy
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/AfterCommitOverride: There can only be one `after_*_commit :notify` hook defined for a model.
end

class Order < ActiveRecord::Base
  after_commit :sync, on: :create
  after_update_commit :sync
  ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/AfterCommitOverride: There can only be one `after_*_commit :sync` hook defined for a model.
  after_commit :sync, on: :destroy
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/AfterCommitOverride: There can only be one `after_*_commit :sync` hook defined for a model.
end
