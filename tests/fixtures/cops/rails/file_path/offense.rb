Rails.root.join("app", "models")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

Rails.root.join("config", "locales", "en.yml")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

Rails.root.join("db", "migrate")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

File.join(Rails.root, "config", "initializers", "action_mailer.rb")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.

File.join(Rails.root, ["app", "models"])
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.

File.join(Rails.root, %w[app models])
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.

File.join(Rails.root, "app", ["models", "goober"])
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.

::File.join(Rails.root, "app", "models")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.

::Rails.root.join("app", "models", "user.rb")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

File.join(Rails.root, "/app/models", "/user.rb")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.

"#{Rails.root}/app/models/goober"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

"#{Rails.root.join('tmp', user.id, 'icon')}.png"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

system "rm -rf #{Rails.root}/foo/bar"
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

File.join(Rails.root.to_s, "lib", "captcha")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.

File.join(::Rails.root.to_s, "app", "models")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.

File.join(flag.present? ? flag : Rails.root.to_s, "lib", "captcha")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.

Pathname.new(Rails.root).join("db", "migrate_sql")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

"#{path.relative_path_from(Rails.root)}.png"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to')`.

File.join((Rails.root || "."), "config", "textcaptcha.yml")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.

File.join(::Rails.root || '', 'config')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FilePath: Prefer `Rails.root.join('path/to').to_s`.
