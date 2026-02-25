# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.add_runtime_dependency 'foo'
       ^^^^^^^^^^^^^^^^^^^^^^ Gemspec/AddRuntimeDependency: Use `add_dependency` instead of `add_runtime_dependency`.
  spec.add_runtime_dependency 'bar', '~> 1.0'
       ^^^^^^^^^^^^^^^^^^^^^^ Gemspec/AddRuntimeDependency: Use `add_dependency` instead of `add_runtime_dependency`.
  s.add_runtime_dependency 'baz'
    ^^^^^^^^^^^^^^^^^^^^^^ Gemspec/AddRuntimeDependency: Use `add_dependency` instead of `add_runtime_dependency`.
end
