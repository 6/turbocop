# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.test_files = ['test/test_helper.rb']
       ^^^^^^^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `test_files` in gemspec.
  spec.date = '2024-01-01'
end

Gem::Specification.new do |s|
  s.name = 'example'
  s.rubygems_version = '3.0'
    ^^^^^^^^^^^^^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `rubygems_version` in gemspec.
end

Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.test_files += Dir.glob('test/**/*')
       ^^^^^^^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `test_files` in gemspec.
end

Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.specification_version = 4
       ^^^^^^^^^^^^^^^^^^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `specification_version` in gemspec.
  spec.rubygems_version = '3.0'
end

Gem::Specification.new do |s|
  s.name = "example"
  s.version = "1.0"
  s.files = `git ls-files`.split("\n")
  s.test_files = `git ls-files -- {test,spec}/*`.split("\n")
    ^^^^^^^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `test_files` in gemspec.
  s.require_paths = ["lib"]
end

Gem::Specification.new do |spec|
  spec.name = "example"
  spec.files = `git ls-files`.split($/)
  spec.test_files = spec.files.grep(%r{^(test|spec)/})
       ^^^^^^^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `test_files` in gemspec.
  spec.require_paths = ["lib"]
end

Gem::Specification.new do |s|
  s.name = "example".freeze
  s.version = "1.0"
  s.required_rubygems_version = Gem::Requirement.new(">= 0".freeze) if s.respond_to? :required_rubygems_version=
  s.require_paths = ["lib".freeze]
  s.date = "2024-01-01"
    ^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `date` in gemspec.
  s.rubygems_version = "3.3.26".freeze
end
