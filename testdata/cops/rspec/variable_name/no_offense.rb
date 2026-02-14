let(:user_name) { 'Adam' }
let(:user_email) { 'adam@example.com' }
let(:age) { 20 }
let!(:record) { create(:record) }
subject(:result) { described_class.new }
let(:items_list) { [1, 2, 3] }
