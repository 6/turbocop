# nitrocop-filename: spec/models/user_spec.rb
RSpec.describe User, type: :model do
                     ^^^^^^^^^^^^ RSpecRails/InferredSpecType: Remove redundant spec type.
end

describe User, type: :model do
               ^^^^^^^^^^^^ RSpecRails/InferredSpecType: Remove redundant spec type.
end

RSpec.describe User, other: true, type: :model do
                                  ^^^^^^^^^^^^ RSpecRails/InferredSpecType: Remove redundant spec type.
end
