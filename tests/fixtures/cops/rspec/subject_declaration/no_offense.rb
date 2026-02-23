subject { 'some subject' }
subject(:named_subject) { 'some subject' }
let(:user) { create(:user) }
let!(:item) { create(:item) }
subject!(:my_subject) { 'some subject' }
let(:something) { 'value' }
