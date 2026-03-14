RSpec.describe Foo do
  it 'is short enough' do
    expect(1).to eq(1)
    expect(2).to eq(2)
  end

  it 'has exactly five lines' do
    a = 1
    b = 2
    c = 3
    d = 4
    expect(a + b + c + d).to eq(10)
  end

  it { expect(true).to be(true) }

  specify do
    expect(subject).to be_valid
  end

  # Example with exactly 5 lines (the default Max)
  it 'uses multi-line code at limit' do
    a = setup_data
    b = process(a)
    c = transform(b)
    d = finalize(c)
    expect(d).to be_valid
  end

  # Comment-only lines do not count (CountComments defaults to false)
  it 'has comments that do not count' do
    # setup comment
    a = setup_data
    # processing comment
    b = process(a)
    # transform comment
    c = transform(b)
    d = finalize(c)
    expect(d).to be_valid
  end

  # Calls with receiver are not RSpec examples
  config.it 'is not an example' do
    a = 1
    b = 2
    c = 3
    d = 4
    e = 5
    f = a + b + c + d + e
    expect(f).to eq(15)
  end

  # Blocks with numbered parameters (_1) are numblock in Parser gem,
  # which on_block does not match — RuboCop intentionally skips them
  it do
    _1.setup
    _1.validate
    _1.process
    _1.transform
    _1.finalize
    _1.verify
  end

  # Heredoc inside a block wrapper: when the body is a single block call
  # containing a heredoc, RuboCop's source_from_node_with_heredoc excludes
  # the block's closing `end` keyword (it's not a descendant node).
  # Lines: Dir.chdir(1) + content=heredoc(1) + body(1) + END(1) + blank(skip) + expect(1) = 5
  it 'example with heredoc in single-statement block wrapper' do
    Dir.chdir('/tmp') do
      content = to_pdf <<~END, analyze: :image
      image::square.png[pdfwidth=1in]
      END

      expect(content).to have_size 1
    end
  end
end
