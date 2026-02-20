# turbocop-filename: my_class_spec.rb
describe MyClass do
  it 'works' do
    expect(true).to eq(true)
  end
end

shared_examples_for 'foo' do
  it 'does stuff' do
    expect(1).to eq(1)
  end
end
