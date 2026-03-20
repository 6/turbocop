describe 'test' do; end
context 'test' do; end
it 'test' do; end
specify 'test' do; end
example 'test' do; end
feature 'test' do; end
let(:fit) { Tax.federal_income_tax }
let(:fit_id) { fit.id }
analyzer.fit(x)
expect { analyzer.fit(Numo::DFloat.new(3, 2).rand) }.to raise_error(ArgumentError)
let(:copied) { Marshal.load(Marshal.dump(analyzer.fit(x))) }
expect { dummy_class.fit }.to raise_error(NotImplementedError)
thing.focus
obj.fdescribe(arg)
