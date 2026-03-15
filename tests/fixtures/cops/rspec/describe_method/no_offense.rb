describe Some::Class do
end

describe Some::Class, '.class_method' do
end

describe Some::Class, '#instance_method' do
end

describe Some::Class, :config do
end

# Nested describe with non-method string should NOT be flagged (only top-level)
describe Chef::Provider::Ifconfig do
  describe Chef::Provider::Ifconfig, "action_delete" do
    it "should delete interface if it exists" do
    end
  end
end

# Nested describe inside context
describe MyClass do
  context "when something" do
    describe MyClass, "some behavior" do
    end
  end
end

# Nested inside shared_examples
describe SomeService do
  shared_examples "common behavior" do
    describe SomeService, "delegation" do
    end
  end
end

# String first argument with method-like second arg — no offense
RSpec.describe "MyService", ".call" do
end

RSpec.describe "MyService", "#process" do
end
