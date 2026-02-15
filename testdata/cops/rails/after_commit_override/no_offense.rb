class User < ActiveRecord::Base
  after_commit :do_something
end

class Post < ActiveRecord::Base
  after_create_commit :notify
end

class Order < ActiveRecord::Base
  after_create_commit :log_create
  after_update_commit :log_update
  after_destroy_commit :log_destroy
end
