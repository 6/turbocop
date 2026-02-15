ENV['SECRET_KEY']
^^^^^^^^^^^^^^^^^ Rails/EnvironmentVariableAccess: Use `ENV.fetch('SECRET_KEY')` instead of `ENV['SECRET_KEY']` for safer access.
ENV["DATABASE_URL"]
^^^^^^^^^^^^^^^^^^^ Rails/EnvironmentVariableAccess: Use `ENV.fetch('DATABASE_URL')` instead of `ENV['DATABASE_URL']` for safer access.
ENV['REDIS_URL']
^^^^^^^^^^^^^^^^ Rails/EnvironmentVariableAccess: Use `ENV.fetch('REDIS_URL')` instead of `ENV['REDIS_URL']` for safer access.
::ENV['API_KEY']
^^^^^^^^^^^^^^^^ Rails/EnvironmentVariableAccess: Use `ENV.fetch('API_KEY')` instead of `ENV['API_KEY']` for safer access.
