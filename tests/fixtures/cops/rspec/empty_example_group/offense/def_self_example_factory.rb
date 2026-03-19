# Example group where examples are created dynamically via def self.method
# RuboCop considers this empty because examples? doesn't descend into defs
describe 'interpolation' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyExampleGroup: Empty example group detected.
  def self.assert_interpolates(name, code, expected)
    example(name) { expect(wrap code).to eq expected }
  end

  assert_interpolates 'backtick syscall', '`echo`', '<`echo`>'
end
