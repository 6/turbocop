describe MyClass do
  before { @foo = [] }
  it { expect(@foo).to be_empty }
              ^^^^ RSpec/InstanceVariable: Avoid instance variables - use let, a method call, or a local variable (if possible).
  it { expect(@bar).to be_empty }
              ^^^^ RSpec/InstanceVariable: Avoid instance variables - use let, a method call, or a local variable (if possible).
end

# Reads inside shared examples are flagged
shared_examples 'shared example' do
  it { expect(@foo).to be_empty }
              ^^^^ RSpec/InstanceVariable: Avoid instance variables - use let, a method call, or a local variable (if possible).
end

# Multiple reads in different example blocks
describe AnotherClass do
  before { @app = create(:app) }
  it 'reads in example' do
    expect(@app.name).to eq('test')
           ^^^^ RSpec/InstanceVariable: Avoid instance variables - use let, a method call, or a local variable (if possible).
  end
  it 'also reads' do
    expect(@app).to be_valid
           ^^^^ RSpec/InstanceVariable: Avoid instance variables - use let, a method call, or a local variable (if possible).
  end
end
