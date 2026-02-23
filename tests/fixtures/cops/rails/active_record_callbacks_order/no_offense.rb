class User < ApplicationRecord
  before_validation :normalize
  after_validation :check
  before_save :prepare
  after_save :do_something
  after_commit :notify
end

class Widget < ApplicationRecord
  before_save :prepare_cache_invalidation!
  before_destroy :prepare_cache_invalidation!
  after_commit :invalidate_cache!
end

class Article < ApplicationRecord
  after_initialize :set_defaults
  before_validation :normalize
  before_destroy :cleanup
  after_destroy :remove_cache
  after_commit :notify_subscribers
end

class Token < ApplicationRecord
  after_create { do_something }
  before_validation { self.email = email.downcase }
  before_save do
    self.token ||= SecureRandom.hex
  end
end
