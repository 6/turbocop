let(:user_name) { 'Adam' }
let(:email) { 'test@example.com' }
let!(:count) { 42 }
subject(:result) { described_class.new }
let(:items) { [1, 2, 3] }
let!(:record) { create(:record) }
