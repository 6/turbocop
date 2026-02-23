it 'test', focus: true do
           ^^^^^^^^^^^ RSpec/Focus: Focused spec found.
end
describe 'test', :focus do
                 ^^^^^^ RSpec/Focus: Focused spec found.
end
context 'test', focus: true do
                ^^^^^^^^^^^ RSpec/Focus: Focused spec found.
end
fit 'test' do
^^^^^^^^^^ RSpec/Focus: Focused spec found.
end
fdescribe 'test' do
^^^^^^^^^^^^^^^^ RSpec/Focus: Focused spec found.
end
::RSpec.describe 'test', :focus do
                         ^^^^^^ RSpec/Focus: Focused spec found.
end
