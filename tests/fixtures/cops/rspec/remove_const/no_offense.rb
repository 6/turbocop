it 'uses stub_const' do
  stub_const("SomeConstant", double)
end

before do
  allow(Object).to receive(:const_get).and_return(double)
end

it 'calls send with other methods' do
  Object.send(:some_method, :arg)
end
