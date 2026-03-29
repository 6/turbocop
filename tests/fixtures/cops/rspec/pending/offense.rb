# x-prefixed example groups (with or without block)
xcontext 'test' do
^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end
xdescribe 'test' do
^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end
xfeature 'test' do
^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end

# x-prefixed examples
xit 'test' do
^^^^^^^^^^ RSpec/Pending: Pending spec found.
end
xspecify 'test' do
^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end
xexample 'test' do
^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end
xscenario 'test' do
^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end

# pending/skip as example-level calls
pending 'test' do
^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end
skip 'test' do
^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end

# standalone skip/pending inside examples
it 'test' do
  skip
  ^^^^ RSpec/Pending: Pending spec found.
end
it 'test' do
  pending
  ^^^^^^^ RSpec/Pending: Pending spec found.
end

# skip/pending with a reason string (no block)
it 'test' do
  skip 'not implemented yet'
  ^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end

# symbol metadata on examples
it 'test', :skip do
^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end
it 'test', :pending do
^^^^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end

# keyword metadata on examples
it 'test', skip: true do
^^^^^^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end
it 'test', skip: 'skipped because of being slow' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end

# symbol metadata on example groups
RSpec.describe 'test', :skip do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end
context 'test', :pending do
^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end

# keyword metadata on example groups
describe 'test', skip: true do
^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
end

# examples without bodies (pending by definition)
it 'test'
^^^^^^^^^ RSpec/Pending: Pending spec found.
specify 'test'
^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
example 'test'
^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.

# examples with only block-pass args are still body-less in RuboCop
it 'uses a proc body', &(proc do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Pending: Pending spec found.
  expect(true).to be(true)
end)

it(&example)
^ RSpec/Pending: Pending spec found.

it(&example)
^ RSpec/Pending: Pending spec found.

it(&example)
^ RSpec/Pending: Pending spec found.

it(&example)
^ RSpec/Pending: Pending spec found.

super { it(&block) }
        ^ RSpec/Pending: Pending spec found.
