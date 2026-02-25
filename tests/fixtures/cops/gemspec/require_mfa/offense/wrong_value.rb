# nitrocop-filename: example.gemspec
# nitrocop-expect: 3:43 Gemspec/RequireMFA: `metadata['rubygems_mfa_required']` must be set to `'true'`.
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.metadata['rubygems_mfa_required'] = 'false'
end
