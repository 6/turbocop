describe MyClass do
  before { @foo = [] }
           ^^^^ RSpec/InstanceVariable: Avoid instance variables - use let, a method call, or a local variable (if possible).
  it { expect(@foo).to be_empty }
              ^^^^ RSpec/InstanceVariable: Avoid instance variables - use let, a method call, or a local variable (if possible).
  it { expect(@bar).to be_empty }
              ^^^^ RSpec/InstanceVariable: Avoid instance variables - use let, a method call, or a local variable (if possible).
end
