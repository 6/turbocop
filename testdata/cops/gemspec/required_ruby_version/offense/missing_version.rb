# rblint-filename: example.gemspec
# rblint-expect: 1:0 Gemspec/RequiredRubyVersion: `required_ruby_version` should be set in gemspec.
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.summary = 'An example gem'
end
