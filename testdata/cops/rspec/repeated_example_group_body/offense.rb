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
