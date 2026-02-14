# rblint-filename: my_class/foo_spec.rb.rb
# rblint-expect: 1:0 RSpec/SpecFilePathSuffix: Spec path should end with `_spec.rb`.
describe MyClass, '#foo' do; end
