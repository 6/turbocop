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

shared_examples 'sentinel support' do
  prefix = 'redis'
  ^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  context 'when configuring' do
    around do |example|
      ClimateControl.modify("#{prefix}_PASSWORD": 'pass') { example.run }
    end
  end
end

describe Whatsapp::SendOnWhatsappService do
  template_params = { name: 'sample_shipping_confirmation' }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  describe '#perform' do
    context 'when a valid message' do
      it 'sends template' do
        message = create(:message, additional_attributes: { template_params: template_params })
        described_class.new(message: message).perform
      end
    end
  end
end
