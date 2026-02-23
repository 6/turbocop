describe 'non-indexed lets' do
  let(:user) { create(:user) }
  let(:admin) { create(:admin) }
  let(:first_item) { create(:item) }
  let(:last_item) { create(:item) }
  let(:primary_account) { create(:account) }
  let(:secondary_account) { create(:account) }
end

# Single indexed let without a matching base is OK (group size = 1 <= Max)
describe 'single indexed let' do
  let(:target_account) { create(:account) }
  let(:target_account2) { create(:account) }
end
