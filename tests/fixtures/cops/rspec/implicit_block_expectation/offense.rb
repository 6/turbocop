# proc {} subject
describe 'proc subject' do
  subject { proc { do_something } }
  it { is_expected.to be_valid }
       ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
end

# lambda {} subject
describe 'lambda subject' do
  subject { lambda { do_something } }
  it { is_expected.to eq(result) }
       ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
end

# Proc.new {} subject
describe 'proc new subject' do
  subject { Proc.new { do_something } }
  it { is_expected.to change { something }.to(new_value) }
       ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
end

# lambda subject inherited from outer group
describe 'outer' do
  subject { lambda { boom } }
  context 'inner' do
    it { is_expected.to change { something }.to(new_value) }
         ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
  end
end

# should / should_not with lambda subject
describe 'should variants' do
  subject { proc { boom } }
  it { should change { something }.to(new_value) }
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
  it { should_not change { something }.to(new_value) }
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
end

# Multiple examples with lambda subject
describe 'multiple' do
  subject { lambda { described_class.run(args) } }
  context 'missing file' do
    let(:file) { 'missing.rb' }
    it { is_expected.to terminate.with_code(1) }
         ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
  end
  context 'unchanged file' do
    let(:file) { 'spec/fixtures/valid' }
    it { is_expected.to terminate }
         ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
  end
end

# Custom method (its_call) with is_expected inside a lambda-subject group
describe 'custom method block' do
  subject { proc { process(input) } }
  its_call('value') { is_expected.to ret([result]) }
                      ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
end

# Custom method with should inside lambda-subject group
describe 'custom should' do
  subject { lambda { run_action } }
  its_call('arg') { should change { counter }.by(1) }
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
end

# Custom method inheriting lambda subject from parent group
describe 'inherited subject' do
  subject { Proc.new { execute } }
  context 'nested' do
    its_call('test') { is_expected.to terminate }
                       ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
  end
end

# RSpec.describe with explicit receiver — top-level example group with RSpec. prefix
RSpec.describe 'with receiver' do
  subject { lambda { process(source) } }
  its_call('value') { is_expected.to ret([result]) }
                      ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
end

# RSpec.describe with lambda subject and regular it block
RSpec.describe 'receiver with it' do
  subject { proc { boom } }
  it { is_expected.to change { something }.to(new_value) }
       ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
end

# RSpec.describe with nested context inheriting lambda subject
RSpec.describe 'receiver nested' do
  subject { Proc.new { execute } }
  context 'inner' do
    it { is_expected.to terminate }
         ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
  end
end

# subject! with proc
describe 'eager' do
  subject! { proc { boom } }
  it { is_expected.to terminate }
       ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
end

# Named subject with lambda
describe 'named' do
  subject(:action) { lambda { boom } }
  it { is_expected.to change { something }.to(new_value) }
       ^^^^^^^^^^^ RSpec/ImplicitBlockExpectation: Avoid implicit block expectations.
end
