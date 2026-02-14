it 'does something' do
  skip 'not yet implemented' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SkipBlockInsideExample: Don't pass a block to `skip` inside examples.
  end
end
specify 'another test' do
  skip 'wip' do
  ^^^^^^^^^^^^^^ RSpec/SkipBlockInsideExample: Don't pass a block to `skip` inside examples.
  end
end
it 'third example' do
  skip 'todo' do
  ^^^^^^^^^^^^^^^ RSpec/SkipBlockInsideExample: Don't pass a block to `skip` inside examples.
    some_code
  end
end
