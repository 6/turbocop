# Variable used in before hook
describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  before { user.update(admin: true) }
end

# Variable used in it block
describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it 'updates the user' do
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end

# Variable used in let
describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  let(:my_user) { user }
end

# Variable used in subject
describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  subject { user }
end

# Variable used as it_behaves_like second argument
describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it_behaves_like 'some example', user
end

# Variable used as part of it_behaves_like argument (array)
describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it_behaves_like 'some example', [user, user2]
end

# Block parameter reassigned outside example scope
shared_examples 'some examples' do |subject|
  subject = SecureRandom.uuid
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it 'renders the subject' do
    expect(mail.subject).to eq(subject)
  end
end

# Variable used in interpolation inside example block body
describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it 'does something' do
    expect("foo_#{user.name}").to eq("foo_bar")
  end
end

# Variable used in both description and block body
describe SomeClass do
  article = foo ? 'a' : 'the'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it "updates #{article} user" do
    user.update(preferred_article: article)
  end
end

# Variable used in nested context's example
describe SomeClass do
  template_params = { name: 'sample_confirmation' }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  describe '#perform' do
    context 'when valid' do
      it 'sends template' do
        message = create(:message, params: template_params)
        described_class.new(message: message).perform
      end
    end
  end
end

# Variable used in nested context's around hook
shared_examples 'sentinel support' do
  prefix = 'redis'
  ^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  context 'when configuring' do
    around do |example|
      ClimateControl.modify("#{prefix}_PASSWORD": 'pass') { example.run }
    end
  end
end

# Variable used in skip metadata AND block body
describe SomeClass do
  skip_message = 'not yet implemented'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it 'does something', skip: skip_message do
    puts skip_message
  end
end

# Variable used as include_context non-first argument
describe SomeClass do
  config = { key: 'value' }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  include_context 'shared setup', config
end

# Variable used inside include_context block
describe SomeClass do
  payload = build(:payload)
  ^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  include_context 'authenticated' do
    let(:data) { payload }
  end
end

# Variable used in it block AND reassigned after use
describe SomeClass do
  user = create(:user)
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it 'updates the user' do
    expect { user.update(admin: true) }.to change(user, :updated_at)
    user = create(:user)
  end
end

# Variable assigned outside describe block, used in before hook
user = create(:user)
^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

describe SomeClass do
  before { user.update(admin: true) }
end

# Variable assigned outside describe block, used in example
record = build(:record)
^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

RSpec.describe SomeClass do
  it 'validates the record' do
    expect(record).to be_valid
  end
end
