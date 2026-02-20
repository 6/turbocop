# turbocop-filename: example.gemspec
# turbocop-expect: 1:0 Gemspec/RequireMFA: `rubygems_mfa_required` must be set to `'true'` in gemspec metadata.
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
end
