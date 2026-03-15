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

# Variable reassigned before use inside example (VariableForce scoping)
describe SomeClass do
  user = create(:user)

  it 'updates the user' do
    user = create(:user)
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end

# Variable used only as first include_context argument (context name)
describe SomeClass do
  ctx = condition ? 'admin context' : 'user context'
  include_context ctx
end

# Variable used in interpolated string for include_context argument
describe SomeClass do
  role = 'admin'
  include_context 'shared setup', "#{role} context"
end

# Variable reassigned inside begin block before use (VariableForce)
describe SomeClass do
  user = create(:user)

  it 'updates the user' do
    begin
      user = create(:other_user)
      expect(user).to be_valid
    end
  end
end

# Variable used only as first arg to include_examples (the shared group name)
describe SomeClass do
  name = condition ? 'admin' : 'user'
  include_examples name
end

# Variable used only as first arg to it_should_behave_like
describe SomeClass do
  behavior = condition ? 'creates record' : 'updates record'
  it_should_behave_like behavior
end

# Variable overwritten in nested context — outer assignment dead, not used in examples
# The outer assignment's value is never read by any example scope; the variable
# is only used at group level.
describe Outer do
  config = { default: true }
  validate(config)

  context 'custom config' do
    it 'does something' do
      expect(1).to eq(1)
    end
  end
end

# Variable assigned inside iterator block param, NOT a group-level assignment
describe SomeClass do
  items.each do |item|
    item = transform(item)
    process(item)
  end

  it 'works' do
    expect(result).to eq(true)
  end
end

# Operator-assign at group level, variable NOT used in example scope
describe SomeClass do
  counter = 0
  counter += items.size

  it 'does something unrelated' do
    expect(1 + 2).to eq(3)
  end
end

# File-level variable reassigned at group scope before any example — file-level value is dead
# RuboCop's VariableForce tracks that the group-level reassignment shadows the file-level one.
user = create(:user)

describe SomeClass do
  user = create(:other_user)

  it 'works' do
    use(user)
  end
end
