RSpec.describe User do
  let(:userName) { 'Adam' }
      ^^^^^^^^^ RSpec/VariableName: Use snake_case for variable names.
  let(:UserName) { 'Adam' }
      ^^^^^^^^^ RSpec/VariableName: Use snake_case for variable names.
  let(:userAge) { 20 }
      ^^^^^^^^ RSpec/VariableName: Use snake_case for variable names.
  subject(:==) { event == other }
          ^^^ RSpec/VariableName: Use snake_case for variable names.
end
