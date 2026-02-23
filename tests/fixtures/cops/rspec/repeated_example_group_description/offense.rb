describe 'doing x' do
^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated describe block description on line(s) [5]
  it { something }
end

describe 'doing x' do
^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated describe block description on line(s) [1]
  it { other }
end

context 'when awesome case' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated context block description on line(s) [13]
  it { thing }
end

context 'when awesome case' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupDescription: Repeated context block description on line(s) [9]
  it { other_thing }
end
