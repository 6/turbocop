RSpec.describe Foo do
  it 'does something' do
    pending 'reason'
    skip 'reason'
  end
  it 'does something', pending: 'reason' do
  end
  it 'does something', skip: 'reason' do
  end
  describe 'something', pending: 'reason' do
  end
  # conditional skip/pending should not be flagged
  it 'does something' do
    pending if jruby?
    skip unless sqlite?
    skip if RUBY_VERSION < '3.0'
    if RUBY_VERSION < '3.0'
      skip
    end
  end
end
# pending/skip outside RSpec context should not be flagged
FactoryBot.define do
  factory :task do
    pending
    skip
    pending { true }
    skip { true }
  end
end
