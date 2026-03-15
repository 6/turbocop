Rails.env.development?
Rails.env.test?
Rails.env.production?
Rails.env
Rails.env == 'production'
Rails.env == "development"
'test' == Rails.env
Rails.env === 'production'
::Rails.env == 'production'
Rails.env.starts_with?("production")
Rails.env.exclude?("development")
Rails.env.include?("test")
Rails.env.end_with?("tion")
Rails.env.match?(/prod/)
