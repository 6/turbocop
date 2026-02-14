File.read(Rails.root.join("config", "database.yml"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: Use `Rails.root.join(...).read` instead of `File.read(Rails.root.join(...))`.

File.exist?(Rails.root.join("tmp", "restart.txt"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: Use `Rails.root.join(...).read` instead of `File.read(Rails.root.join(...))`.

File.delete(Rails.root.join("tmp", "pids", "server.pid"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: Use `Rails.root.join(...).read` instead of `File.read(Rails.root.join(...))`.