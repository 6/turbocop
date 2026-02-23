Rails.env == "production"
^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EnvironmentComparison: Use `Rails.env.production?` instead of comparing `Rails.env`.
Rails.env != "development"
^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EnvironmentComparison: Use `Rails.env.production?` instead of comparing `Rails.env`.
"test" == Rails.env
^^^^^^^^^^^^^^^^^^^ Rails/EnvironmentComparison: Use `Rails.env.production?` instead of comparing `Rails.env`.
::Rails.env == "staging"
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EnvironmentComparison: Use `Rails.env.production?` instead of comparing `Rails.env`.
