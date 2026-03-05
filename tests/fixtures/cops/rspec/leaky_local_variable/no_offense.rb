# Variable assigned inside example
describe SomeClass do
  it 'updates the user' do
    user = create(:user)
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end

# Variable used only as it description
describe SomeClass do
  description = "updates the user"
  it description do
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end

# Variable used only in it description interpolation
describe SomeClass do
  article = foo ? 'a' : 'the'
  it "updates #{article} user" do
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end

# Block parameter used in example (not reassigned)
shared_examples 'some examples' do |subject|
  it 'renders the subject' do
    expect(mail.subject).to eq(subject)
  end
end

# Block keyword parameter used in example
shared_examples 'some examples' do |subject:|
  it 'renders the subject' do
    expect(mail.subject).to eq(subject)
  end
end

# Block parameter reassigned inside example
shared_examples 'some examples' do |subject|
  it 'renders the subject' do
    subject = 'hello'
    expect(mail.subject).to eq(subject)
  end
end

# Two variables same name in different scopes
describe SomeClass do
  let(:my_user) do
    user = create(:user)
    user.flag!
    user
  end

  it 'updates the user' do
    user = create(:user)
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end

# Variable not referenced in any example
describe SomeClass do
  user = create(:user)
  user.flag!

  it 'does something' do
    expect(foo).to eq(bar)
  end
end

# Variable used as first it_behaves_like argument (shared example name)
describe SomeClass do
  examples = foo ? 'definite article' : 'indefinite article'
  it_behaves_like examples
end

# Variable used in interpolation for it_behaves_like argument
describe SomeClass do
  article = foo ? 'a' : 'the'
  it_behaves_like 'some example', "#{article} user"
end

# Variable used in symbol interpolation for it_behaves_like argument
describe SomeClass do
  article = foo ? 'a' : 'the'
  it_behaves_like 'some example', :"#{article}_user"
end

# Block argument shadowed by local variable in iterator
describe SomeClass do
  %i[user user2].each do |user|
    let(user) do
      user = create(:user)
      user.flag!
      user
    end
  end
end

# Outside of a describe block (FactoryBot)
FactoryBot.define :foo do
  bar = 123

  after(:create) do |foo|
    foo.update(bar: bar)
  end
end

# Variable used only in skip metadata
describe SomeClass do
  skip_message = 'not yet implemented'

  it 'does something', skip: skip_message do
    expect(1 + 2).to eq(3)
  end
end

# Variable used only in pending metadata
describe SomeClass do
  pending_message = 'work in progress'

  it 'does something', pending: pending_message do
    expect(1 + 2).to eq(3)
  end
end
