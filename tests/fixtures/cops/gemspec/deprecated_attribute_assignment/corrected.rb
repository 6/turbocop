Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.date = '2024-01-01'
end

Gem::Specification.new do |s|
  s.name = 'example'
end

Gem::Specification.new do |spec|
  spec.name = 'example'
end

Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.rubygems_version = '3.0'
end

Gem::Specification.new do |s|
  s.name = "example"
  s.version = "1.0"
  s.files = `git ls-files`.split("\n")
  s.require_paths = ["lib"]
end

Gem::Specification.new do |spec|
  spec.name = "example"
  spec.files = `git ls-files`.split($/)
  spec.require_paths = ["lib"]
end

Gem::Specification.new do |s|
  s.name = "example".freeze
  s.version = "1.0"
  s.required_rubygems_version = Gem::Requirement.new(">= 0".freeze) if s.respond_to? :required_rubygems_version=
  s.require_paths = ["lib".freeze]
  s.rubygems_version = "3.3.26".freeze
end
