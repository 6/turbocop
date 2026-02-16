task :lint do
  FILES_TO_LINT = Dir['lib/*.rb']
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/ConstantDefinitionInBlock: Do not define constants this way within a block.
end

describe 'making a request' do
  class TestRequest; end
  ^^^^^^^^^^^^^^^^^^^^^^ Lint/ConstantDefinitionInBlock: Do not define constants this way within a block.
end

module M
  extend ActiveSupport::Concern
  included do
    LIST = []
    ^^^^^^^^^ Lint/ConstantDefinitionInBlock: Do not define constants this way within a block.
  end
end
