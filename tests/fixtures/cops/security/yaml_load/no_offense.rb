YAML.safe_load(data)
YAML.parse(data)
Psych.safe_load(data)
obj.load(data)
yaml_load(data)
# With TargetRubyVersion >= 3.1, YAML.load is safe (Psych 4 default)
# This test verifies the cop doesn't fire without the version check
