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

# Variable overwritten at scope level — only last assignment is offense (FP fix)
# The first `result = nil` is dead; only `result = compute()` leaks.
describe SomeClass do
  result = nil
  result = compute()
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it 'checks the result' do
    expect(result).to eq(42)
  end
end

# Variable overwritten with intervening non-reading statement — only last is offense
describe SomeClass do
  count = 0
  do_something
  count = items.size
  ^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it 'has the right count' do
    expect(count).to eq(5)
  end
end

# Variable used via operator-assign (+=) inside example block
describe SomeClass do
  total = 10
  ^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it 'increments the total' do
    total += 5
    expect(total).to eq(15)
  end
end

# Variable used via operator-assign (-=) inside hook
describe SomeClass do
  counter = 100
  ^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  before do
    counter -= 1
  end

  it 'checks counter' do
    expect(counter).to eq(99)
  end
end

# Variable used in interpolated regex inside example
describe SomeClass do
  pattern = 'foo'
  ^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it 'matches the pattern' do
    expect('foobar').to match(/#{pattern}/)
  end
end

# Dead file-level assignments are NOT flagged; only the last unconditional
# assignment before the describe-block reference is live. (dev-sec pattern)
flags = parse_config('/proc/cpuinfo').flags
flags ||= ''
flags = flags.split(' ')
^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

describe '/proc/cpuinfo' do
  it 'Flags should include NX' do
    expect(flags).to include('nx')
  end
end

# Variables inside .each blocks used in nested example scopes
describe "iterator block" do
  [1, 2].each do |v|
    val = v.to_s
    ^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
    context "when val=#{val}" do
      it "works" do
        expect(val).to eq(v.to_s)
      end
    end
  end
end

# File-level variable assigned in if/elsif branches, used in describe block
root_group = 'root'
^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

if os == 'aix'
  root_group = 'system'
  ^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
elsif os == 'freebsd'
  root_group = 'wheel'
  ^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
end

describe SomeClass do
  its('groups') { should include root_group }
end

# Variable assigned in if-condition, used in let block
describe SomeClass do
  specs.each do |spec|
    context spec['name'] do
      if error = spec['error']
         ^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
        let(:expected_error) { error }

        it 'fails' do
          expect { run }.to raise_error(expected_error)
        end
      end
    end
  end
end

# Variable assigned before non-RSpec block containing RSpec.describe
describe SomeClass do
  max_count = 4
  ^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  with_new_environment do
    spec = RSpec.describe "SomeTest" do
      it "test" do
        expect(max_count).to eq(4)
      end
    end

    spec.run
  end
end

# Ruby 3.1 keyword shorthand: `method(url:)` is shorthand for `method(url: url)`
# Prism wraps the value in an ImplicitNode containing a LocalVariableReadNode.
describe "Feed importing" do
  url = "feed02/feed.xml"
  ^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it "imports stories" do
    server = create_server(url:)
  end
end

# Ruby 3.1 keyword shorthand with multiple shorthand args
describe "#update" do
  headers = { "CONTENT_TYPE" => "application/json" }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  it "marks a story as read" do
    put("/stories/#{story.id}", headers:)
  end
end

# Ruby 3.1 keyword shorthand in before hook
describe "fetching" do
  last_fetched = Time.parse("2014-08-12T00:01:00Z")
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

  before do
    create_feed(last_fetched:)
  end
end

# def self.method with variables leaking into example scopes via .each
describe "dynamic cases" do
  def self.define_cases(items)
    items.each do |label, value|
      result = value.upcase
      ^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
      context label do
        it { expect(something).to eq(result) }
      end
    end
  end
end

# def method with variables leaking into RSpec.describe inside a block
describe "instance method" do
  def run_test
    counter = 0
    ^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
    with_new_rspec_environment do
      RSpec.describe "inner" do
        it { expect(counter).to eq(0) }
      end
    end
  end
end

# def self.method with direct example scopes (no wrapping describe)
describe "direct examples in def self" do
  def self.it_is_correct_for(label, expected)
    result = expected.to_s
    ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
    it "is correct for #{label}" do
      expect(compute).to eq(result)
    end
  end
end

# Variable assigned in nested context, used in example interpolation and call
RSpec.describe Database::Multiple, '#multiple' do
  context '#Work with proper query' do
    join_table_name = 'object_query_5'
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
    join_table_column = 'oo_id'
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

    it 'builds the expected SQL' do
      expect("UPDATE #{join_table_name}").to include(
        join_table_column
      )
    end
  end

  context '#Work with linked tables' do
    join_table_name = 'object_query_5'
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
    join_table_column = 'oo_id'
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

    it 'formats linked table updates' do
      expect("JOIN #{join_table_name} ON #{join_table_column}").to include(
        "#{join_table_name}.#{join_table_column}"
      )
    end
  end
end

# File-level conditional assignment used in example
def which(cmd)
  cmd
end

insert_tee_log = '  2>&1 | tee -a vagrant.log ' if which('tee')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

describe 'VM Life Cycle' do
  it 'starts Linux and Windows VM' do
    expect("vagrant up #{insert_tee_log}").to include('tee')
  end
end

# Variable initialized to nil, read in before hook predicate, then used in example
describe 'Puppet Ruby Generator' do
  context 'when generating static code' do
    module_def = nil
    ^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

    before(:each) do
      if module_def.nil?
        module_def = build_module
      end
    end

    it 'keeps the generated module' do
      expect(module_def).not_to be_nil
    end
  end
end

# Same nil-initialization pattern in a separate nested context
describe 'TypeSet generator' do
  context 'when generating static code' do
    module_def = nil
    ^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
    module_def2 = nil

    before(:each) do
      if module_def.nil?
        module_def = build_primary_module
        module_def2 = build_secondary_module
      end
    end

    it 'uses the first generated module' do
      expect(module_def.name).to eq(module_def2.parent_name)
    end
  end
end

# Variable inside shared_examples nested describe, used in example
shared_examples 'inspect unmanaged files' do |base, skip_remote_mounts_test|
  describe '--scope=unmanaged-files' do
    test_tarball = File.join(Machinery::ROOT, '../machinery/spec/definitions/vagrant/unmanaged_files.tgz')
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.

    it 'extracts list of unmanaged files' do
      expect(test_tarball).to include('unmanaged_files.tgz')
    end
  end
end

# Variables assigned inside nested hash expressions at group scope
describe SomeClass do
  schema = {
    const: const_schema = { const: 1 },
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
    required: required_props = %w[a b],
              ^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
    dependentRequired: {
      (p_0 = :foo) => [
       ^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
        p_1 = :bar
        ^^^^^^^^^^ RSpec/LeakyLocalVariable: Do not use local variables defined outside of examples inside of them.
      ]
    }
  }

  validate(schema)

  it 'uses nested assignments' do
    expect(const_schema[:const]).to eq(1)
    expect(required_props).to include('a')
    expect(p_0).to eq(:foo)
    expect(p_1).to eq(:bar)
  end
end
