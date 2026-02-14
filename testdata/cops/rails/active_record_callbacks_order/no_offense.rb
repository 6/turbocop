class User < ApplicationRecord
  before_validation :normalize
  after_validation :check
  before_save :prepare
  after_save :do_something
  after_commit :notify
end
