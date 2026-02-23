describe 'indexed lets' do
  let(:item_1) { create(:item) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:item_2) { create(:item) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `2` in its name. Please give it a meaningful name.
  let(:user1) { create(:user) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:user2) { create(:user) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `2` in its name. Please give it a meaningful name.
end
