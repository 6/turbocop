def short_method
  x = 1
  x = 2
  x = 3
end

def ten_lines
  a = 1
  b = 2
  c = 3
  d = 4
  e = 5
  f = 6
  g = 7
  h = 8
  i = 9
  j = 10
end

def empty_method
end

def one_liner
  42
end

def with_branch
  if true
    1
  else
    2
  end
end

# RuboCop counts a method body that is only a heredoc expression as one line.
# Even with large heredoc content, this should not trigger MethodLength.
def heredoc_only_method
  <<~SQL
    line1
    line2
    line3
    line4
    line5
    line6
    line7
    line8
    line9
    line10
    line11
    line12
    line13
    line14
    line15
    line16
    line17
    line18
    line19
    line20
    line21
    line22
    line23
    line24
    line25
    line26
    line27
    line28
    line29
    line30
  SQL
end

# Multiline params should not count toward body length.
# RuboCop counts only body.source lines, not parameter lines.
def initialize(
  param1: nil,
  param2: nil,
  param3: nil,
  param4: nil,
  param5: nil,
  param6: nil,
  param7: nil,
  param8: nil,
  param9: nil,
  param10: nil
)
  a = param1
  b = param2
  c = param3
end

# define_method with short body (no offense)
define_method(:short_dynamic) do
  a = 1
  b = 2
  c = 3
end

# define_method at exactly Max lines
define_method(:ten_dynamic) do
  a = 1
  b = 2
  c = 3
  d = 4
  e = 5
  f = 6
  g = 7
  h = 8
  i = 9
  j = 10
end

# define_method with brace block
define_method(:brace_dynamic) { |x|
  a = 1
  b = 2
}

# define_method with string name
define_method("string_name") do
  a = 1
end

# Receiver-qualified define_method with short body (no offense)
builder.define_method(:short_generated) do
  a = 1
  b = 2
  c = 3
end

# When a method body contains heredocs, RuboCop's source_from_node_with_heredoc
# computes lines from body.first_line to max descendant last_line, which excludes
# wrapper closing keywords (block `end`s). This method has 11 physical non-blank
# body lines (counting the block end), but RuboCop counts 10 (excluding it).
def test_heredoc_in_block
  in_tmpdir do
    path = current_dir.join("config")
    path.write(<<~TEXT)
      target :app do
        collection_config "test.yaml"
      end
    TEXT
    current_dir.join("test.yaml").write("[]")

    Runner.new.load_config(path: path)
    assert_match(/pattern/, output.string)
  end
end

# Same pattern with <<- heredoc in a block (10 non-blank lines per RuboCop).
def test_indented_heredoc_in_block
  setup do
    config = <<-YAML
      key: value
      other: data
    YAML
    load_config(config)
    validate_config
    process_data
    check_results
    verify_output
  end
end

# Heredoc inside an if/else body: RuboCop's source_from_node_with_heredoc
# excludes the `if` node's own `end` keyword from the line count.
# Physical non-blank body lines: 11 (if, heredoc opener, 5 content, closer,
# raise, end = 11). But RuboCop counts 10 via max descendant end_line.
def validate_range(value)
  if value > 9999
    message = <<~ERROR
      Value is out of range.

      The system will treat this as invalid.
      Please provide a value within bounds.

      To override, set skip_validation to true.
    ERROR
    raise ArgumentError, message
  end
end

# Heredoc inside an ensure block: source_from_node_with_heredoc counts
# body.first_line to max descendant last_line, excluding the ensure `end`.
def write_and_cleanup
  f = Tempfile.new("test")
  f.write <<-RUBY
    config[:name] = "value"
  RUBY
  f.close

  options = "-e test"
  run_command(options, env: { "CFG" => f.path })

  check_output "result"
ensure
  File.unlink(f)
end

# Heredoc inside if/else: RuboCop's source_from_node_with_heredoc uses
# body.each_descendant which excludes the if node itself. Parser has no
# ElseNode wrapper, so the else branch's last_line is the last statement,
# not the `end` keyword. RuboCop counts 10 body lines here (2-11).
def initialize(version = nil, name = nil)
  if version && name
    super(<<~MSG)
      Invalid timestamp for migration file.
      Timestamp must be in form YYYYMMDDHHMMSS.
    MSG
  else
    super(<<~MSG)
      Invalid timestamp for migration.
      Timestamp must be in form YYYYMMDDHHMMSS.
    MSG
  end
end

# Heredoc inside if/else with non-heredoc else branch.
# RuboCop: max descendant = MSG end line, excludes `if` end. 10 body lines.
def render_output(data)
  if data.nil?
    output = <<~HTML
      <div class="empty">
        <p>No data available</p>
      </div>
    HTML
    log_warning(output)
  else
    format_data(data)
  end
end

# Endless method with short multiline body (no offense)
def compact_settings = {
  one: 1,
  two: 2,
  three: 3
}
