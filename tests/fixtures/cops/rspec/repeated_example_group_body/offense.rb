describe 'doing x' do
^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [5]
  it { cool_predicate_method }
end

describe 'doing y' do
^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [1]
  it { cool_predicate_method }
end

context 'when awesome case' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated context block body on line(s) [13]
  it { another_predicate_method }
end

context 'when another awesome case' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated context block body on line(s) [9]
  it { another_predicate_method }
end

describe 'quoting case a' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [21]
  it { expect(subject).to eq('hello') }
end

describe 'quoting case b' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [17]
  it { expect(subject).to eq("hello") }
end

context 'parens case a' do
^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated context block body on line(s) [29]
  it { expect(subject).to eq(1) }
end

context 'parens case b' do
^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated context block body on line(s) [25]
  it { expect(subject).to eq 1 }
end
