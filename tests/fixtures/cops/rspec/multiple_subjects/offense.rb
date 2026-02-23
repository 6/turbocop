describe 'hello there' do
  subject(:foo) { 1 }
  ^^^^^^^^^^^^^^^^^^^ RSpec/MultipleSubjects: Do not set more than one subject per example group
  subject(:bar) { 2 }
  ^^^^^^^^^^^^^^^^^^^ RSpec/MultipleSubjects: Do not set more than one subject per example group
  subject { 3 }
  ^^^^^^^^^^^^^ RSpec/MultipleSubjects: Do not set more than one subject per example group
  subject(:baz) { 4 }

  describe 'baz' do
    subject(:norf) { 1 }
  end
end
