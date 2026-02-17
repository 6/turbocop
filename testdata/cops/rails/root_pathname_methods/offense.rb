File.read(Rails.root.join("config", "database.yml"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).read` instead.

File.exist?(Rails.root.join("tmp", "restart.txt"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).exist?` instead.

File.delete(Rails.root.join("tmp", "pids", "server.pid"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).delete` instead.

File.join(Rails.root, "config", "initializers", "action_mailer.rb")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join` instead of `File.join(Rails.root, ...)`.

Dir.glob(Rails.root.join("db", "**", "*.rb"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).glob` instead.
