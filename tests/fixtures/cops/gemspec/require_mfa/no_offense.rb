# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.metadata['rubygems_mfa_required'] = 'true'
  spec.authors = ['Author']
  spec.summary = 'An example gem'
end

# Also detect hash-style metadata assignment
Gem::Specification.new do |s|
  s.name = 'example2'
  s.metadata = {
    'rubygems_mfa_required' => 'true'
  }
end
