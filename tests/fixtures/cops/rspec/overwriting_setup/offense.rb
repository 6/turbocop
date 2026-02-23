RSpec.describe User do
  let(:a) { a }
  let(:a) { b }
  ^^^^^^^^^^^^^ RSpec/OverwritingSetup: `a` is already defined.
end

RSpec.describe User do
  subject(:a) { a }

  let(:a) { b }
  ^^^^^^^^^^^^^ RSpec/OverwritingSetup: `a` is already defined.
end

RSpec.describe User do
  subject { a }

  let(:subject) { b }
  ^^^^^^^^^^^^^^^^^^^ RSpec/OverwritingSetup: `subject` is already defined.
end
