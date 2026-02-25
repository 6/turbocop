# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.add_development_dependency 'rspec', '~> 3.0'
       ^^^^^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DevelopmentDependencies: Specify development dependencies in `Gemfile` instead of gemspec.
  spec.add_development_dependency 'rubocop'
       ^^^^^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DevelopmentDependencies: Specify development dependencies in `Gemfile` instead of gemspec.
  s.add_development_dependency 'simplecov'
    ^^^^^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DevelopmentDependencies: Specify development dependencies in `Gemfile` instead of gemspec.
end
