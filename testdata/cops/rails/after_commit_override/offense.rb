class User < ActiveRecord::Base
  after_commit :do_first
  after_commit :do_second
  ^^^^^^^^^^^^^^^^^^^^^^^ Rails/AfterCommitOverride: Multiple `after_commit` callbacks may override each other.
end
