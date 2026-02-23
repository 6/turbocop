describe SomeClass do
  it 'updates the user' do
    user = create(:user)
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end
describe SomeClass do
  description = "updates the user"
  it description do
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end
