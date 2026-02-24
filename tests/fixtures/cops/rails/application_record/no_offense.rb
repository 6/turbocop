class User < ApplicationRecord
end

class ApplicationRecord < ActiveRecord::Base
end

class Post < ApplicationRecord
end

# Namespaced ApplicationRecord should not be flagged
class Admin::ApplicationRecord < ActiveRecord::Base
end

# Deeply nested ApplicationRecord
module Tenant
  class ApplicationRecord < ActiveRecord::Base
  end
end
