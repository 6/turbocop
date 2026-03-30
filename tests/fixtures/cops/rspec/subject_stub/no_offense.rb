require "spec_helper"

describe Foo do
  subject(:foo) { described_class.new }

  before do
    allow(other_obj).to receive(:bar).and_return(baz)
  end

  it 'does something' do
    expect(foo.bar).to eq(baz)
  end
end

describe Bar do
  let(:bar) { double }

  before do
    allow(bar).to receive(:baz)
  end
end

RSpec.shared_examples 'a web server adapter' do
  subject(:adapter) { described_class.new(upgrader) }

  let(:upgrader) { instance_double(DockerManager::Upgrader, log: nil) }

  describe '#workers' do
    before do
      allow(adapter).to receive(:master_pid).and_return(1001)
    end
  end
end

# When require is at top level alongside a module wrapper, RuboCop's TopLevelGroup
# does not recurse into the module (begin returns children directly, module is
# not a spec group so it is skipped).
module SomeModule
  describe Builder do
    subject { described_class.new }

    before do
      allow(subject).to receive(:windows?)
    end
  end
end

# Local variable named subject is not the RSpec subject method
describe Agent do
  it 'returns false when failed?' do
    subject = Agent.new(0)
    allow(subject).to receive(:failed?).and_return(true)
    expect(subject.send { nil }).to be false
  end
end

# Subject name redefined by let in same or child scope
RSpec.describe Foo do
  subject(:foo) { described_class.new }

  context 'when foo is redefined by let' do
    let(:foo) { described_class.new }

    before do
      allow(foo).to receive(:active?).and_return(true)
    end
  end
end

# Subject name redefined by let in same scope
RSpec.describe Widget do
  subject(:widget) { described_class.new }
  let(:widget) { described_class.new }

  before do
    allow(widget).to receive(:enabled?).and_return(false)
  end
end

# Stubs inside class methods (def self.) are not flagged — RuboCop's
# find_subject_expectations recurses into :def but not :defs nodes.
describe Runner do
  subject(:runner) { described_class.new(stdout, stderr) }

  let(:stdout) { StringIO.new }
  let(:stderr) { StringIO.new }

  def self.cmds(cmds)
    before { cmds.each { |cmd, str| allow(runner).to receive(:`).with(cmd.to_s).and_return(str) } }
  end
end

# Subject from parent redefined with let in nested context (vendor spec case)
RSpec.describe Service do
  subject(:service) { described_class.new }

  context 'nested context' do
    subject(:record) { service.record }

    let(:service) { described_class.new }

    before do
      allow(service).to receive(:active?).and_return(true)
    end
  end
end

# Derived subject values can stub the collaborator used to compute the subject.
RSpec.describe Project do
  subject { project.classification_progress }

  let(:project) { Project.new }

  before do
    allow(project).to receive(:info_requests).and_return(
      double(count: 3, classified: double(count: 2))
    )
  end
end
