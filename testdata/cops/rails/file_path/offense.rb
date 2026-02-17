Rails.root.join("app", "models")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

Rails.root.join("config", "locales", "en.yml")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

Rails.root.join("db", "migrate")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

File.join(Rails.root, "config", "initializers", "action_mailer.rb")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.
