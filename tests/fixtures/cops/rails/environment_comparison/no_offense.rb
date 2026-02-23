Rails.env.production?
Rails.env.development?
Rails.env.test?
x == "production"
Rails.env.local?
# Non-literal compared to Rails.env should not be flagged
cluster == Rails.env
Rails.env == some_variable
Rails.env != config_value
