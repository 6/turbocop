YAML.load(data)
     ^^^^ Security/YAMLLoad: Prefer `YAML.safe_load` over `YAML.load`.
Psych.load(data)
      ^^^^ Security/YAMLLoad: Prefer `YAML.safe_load` over `YAML.load`.
::YAML.load(x)
       ^^^^ Security/YAMLLoad: Prefer `YAML.safe_load` over `YAML.load`.
