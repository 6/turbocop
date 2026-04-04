class User < ApplicationRecord
  self.ignored_columns = [:real_name]
                          ^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `real_name` from `ignored_columns` because the column does not exist.
end

class User < ApplicationRecord
  self.ignored_columns = ['real_name']
                          ^^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `real_name` from `ignored_columns` because the column does not exist.
end

class User < ApplicationRecord
  self.ignored_columns = [:real_name, :nickname]
                          ^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `real_name` from `ignored_columns` because the column does not exist.
                                      ^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `nickname` from `ignored_columns` because the column does not exist.
end

class Edition < ApplicationRecord
  self.ignored_columns += %w[news_article_type_id]
                             ^^^^^^^^^^^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `news_article_type_id` from `ignored_columns` because the column does not exist.
end

class OffsiteLink < ApplicationRecord
  self.ignored_columns += %w[parent_id parent_type]
                                       ^^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `parent_type` from `ignored_columns` because the column does not exist.
end

class Account < ApplicationRecord
  self.ignored_columns += %w(
    devices_url
    ^^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `devices_url` from `ignored_columns` because the column does not exist.
    hub_url
    ^^^^^^^ Rails/UnusedIgnoredColumns: Remove `hub_url` from `ignored_columns` because the column does not exist.
    remote_url
    ^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `remote_url` from `ignored_columns` because the column does not exist.
    salmon_url
    ^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `salmon_url` from `ignored_columns` because the column does not exist.
    secret
    ^^^^^^ Rails/UnusedIgnoredColumns: Remove `secret` from `ignored_columns` because the column does not exist.
    subscription_expires_at
    ^^^^^^^^^^^^^^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `subscription_expires_at` from `ignored_columns` because the column does not exist.
    trust_level
    ^^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `trust_level` from `ignored_columns` because the column does not exist.
  )
end
