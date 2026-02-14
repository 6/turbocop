class User < ActiveRecord::Base
  after_commit :do_something
end

class Post < ActiveRecord::Base
  after_create_commit :notify
end
