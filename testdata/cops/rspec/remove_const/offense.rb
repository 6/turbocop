it 'does something' do
  Object.send(:remove_const, :SomeConstant)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RemoveConst: Do not use remove_const in specs. Consider using e.g. `stub_const`.
end

before do
  SomeClass.send(:remove_const, :SomeConstant)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RemoveConst: Do not use remove_const in specs. Consider using e.g. `stub_const`.
end

it 'removes via __send__' do
  NiceClass.__send__(:remove_const, :SomeConstant)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RemoveConst: Do not use remove_const in specs. Consider using e.g. `stub_const`.
end
