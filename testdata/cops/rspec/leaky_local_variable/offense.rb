describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  before { user.update(admin: true) }
end

describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it 'updates the user' do
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end

describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  let(:my_user) { user }
end
