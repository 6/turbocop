RSpec.describe Foo do
  it do
    expect(subject.a).to eq(3)
    expect(subject.b).to eq(4)
  end
end

RSpec.describe Bar do
  it do
    expect { subject }.to change { Bar.count }
    expect(subject.b).to eq(4)
  end
end

# Chained subject calls are not flagged (subject is a receiver)
RSpec.describe Baz do
  it do
    expect(subject.reblogs_count).to eq(1)
    expect { subject.destroy }.to_not raise_error
  end
end

# subject as argument to expect() — parent is a send node, not flagged
RSpec.describe Qux do
  it "allows voting" do
    expect(subject).not_to be_allowed(some_resource)
    other_resources.each do |resource|
      expect(subject).to be_allowed(resource)
    end
  end
end

# subject as argument inside blocks should not be flagged
RSpec.describe Quux do
  it "checks results" do
    expect(subject).to eq(expected)
    items.each do |item|
      expect(subject).to include(item)
    end
  end
end

# subject as argument to create/other methods inside expect blocks
RSpec.describe Corge do
  it do
    expect { create(:item, owner: subject) }.to change { Item.count }
    expect { create(:item, subject) }.to not_change { Item.count }
  end
end

# Named subject used chained (as receiver) should not count as a repeat of the bare name
# Pattern: subject(:track) with `metric.values` in change block — metric is another subject
# but used chained, so it should not be counted as a flaggable call. Only 1 bare `track` call.
RSpec.describe Metric do
  subject(:metric) { described_class.new(name) }

  describe '#track' do
    subject(:track) { metric.track(value) }

    it 'tracks the value' do
      expect { track }.to change { metric.values }
    end

    context 'tracking again' do
      it 'updates values' do
        metric.track(value)
        expect { track }.to change { metric.values }
      end
    end
  end
end

# Single subject call with non-subject expressions before it
RSpec.describe Writer do
  subject(:write) { writer.write(event) }

  it "starts a worker thread" do
    expect(writer.buffer).to receive(:push).with(event)
    expect { write }.to change { writer.running? }
  end
end

# Single subject call in expect block with non-subject call before it
RSpec.describe Cleanup do
  it 'does not delete root dir' do
    expect(File.directory?(factory.root_dir)).to be(true)
    expect { subject }.not_to change { File.directory?(factory.root_dir) }
  end
end

# Different named subjects — each used once is not an offense
RSpec.describe Grault do
  subject { do_something_else }
  subject(:bar) { do_something }

  it do
    expect { bar }.to not_change { Grault.count }
    expect { subject }.to not_change { Grault.count }
  end
end

# RuboCop's TopLevelGroup mixin does not descend into a module-wrapped spec
# when the file also has sibling top-level statements like `require`.
require "rails_helper"

module Maintenance
  RSpec.describe Wrapped do
    describe "#process" do
      subject(:process) { described_class.process(procedure) }

      it do
        expect { process }.not_to change { foo }
        expect { process }.not_to change { bar }
      end
    end
  end
end
