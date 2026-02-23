# turbocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.name = 'other'
       ^^^^ Gemspec/AttributeAssignment: Attribute `name` is already set on line 2.
  spec.summary = 'Summary'
  spec.summary = 'Other summary'
       ^^^^^^^ Gemspec/AttributeAssignment: Attribute `summary` is already set on line 5.
  spec.version = '2.0'
       ^^^^^^^ Gemspec/AttributeAssignment: Attribute `version` is already set on line 3.
end
