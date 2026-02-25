# turbocop-filename: example.gemspec
# turbocop-expect: 3:0 Gemspec/RequireMFA: `metadata['rubygems_mfa_required']` must be set to `'true'`.
# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
end
