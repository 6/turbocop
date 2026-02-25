# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.test_files = ['test/test_helper.rb']
       ^^^^^^^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `test_files` in gemspec.
  spec.date = '2024-01-01'
       ^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `date` in gemspec.
  spec.specification_version = 4
       ^^^^^^^^^^^^^^^^^^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `specification_version` in gemspec.
  spec.rubygems_version = '3.0'
       ^^^^^^^^^^^^^^^^ Gemspec/DeprecatedAttributeAssignment: Do not set `rubygems_version` in gemspec.
end
