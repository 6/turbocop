# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.add_dependency 'foo'
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_runtime_dependency 'bar'
       ^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_development_dependency 'baz'
       ^^^^^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_dependency %q<os>
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_runtime_dependency %q(parser)
       ^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_development_dependency %q[minitest]
       ^^^^^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_dependency 'interp', "~> #{VERSION}"
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
end
