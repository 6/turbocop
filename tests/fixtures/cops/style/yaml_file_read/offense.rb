YAML.load(File.read('config.yml'))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YAMLFileRead: Use `YAML.load_file` instead of `YAML.load` with `File.read`.

YAML.safe_load(File.read('data.yml'))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YAMLFileRead: Use `YAML.safe_load_file` instead of `YAML.safe_load` with `File.read`.

YAML.parse(File.read('input.yml'))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YAMLFileRead: Use `YAML.parse_file` instead of `YAML.parse` with `File.read`.
