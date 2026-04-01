def test
  if something
  ^^ Style/GuardClause: Use a guard clause (`return unless something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def test
  unless something
  ^^^^^^ Style/GuardClause: Use a guard clause (`return if something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def test
  other_work
  if something
  ^^ Style/GuardClause: Use a guard clause (`return unless something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def test
  other_work
  unless something
  ^^^^^^ Style/GuardClause: Use a guard clause (`return if something`) instead of wrapping the code inside a conditional expression.
    work
  end
end

def complete_expression?(expression)
  original_complete_expression?(expression)
rescue SyntaxError => e
  if e.message =~ /expected a `.*` to close the .* literal/ || e.message =~ /unterminated list/
  ^^ Style/GuardClause: Use a guard clause (`raise e unless e.message =~ /expected a `.*` to close the .* literal/ || e.message =~ /unterminated list/`) instead of wrapping the code inside a conditional expression.
    false
  else
    raise e
  end
end

def to_tmdb_struct(klass = Tmdb::Struct)
  if descendent_of_tmdb_struct?(klass)
  ^^ Style/GuardClause: Use a guard clause (`raise Tmdb::Error, 'Tried to convert to a non Tmdb::Struct object' unless descendent_of_tmdb_struct?(klass)`) instead of wrapping the code inside a conditional expression.
    klass.new(self)
  else
    raise Tmdb::Error, 'Tried to convert to a non Tmdb::Struct object'
  end
end

def get(parsed_exception_rs, e)
  if parsed_exception_rs['status_message'].present?
  ^^ Style/GuardClause: Use a guard clause (`raise Tmdb::Error, parsed_exception_rs['status_message'] if parsed_exception_rs['status_message'].present?`) instead of wrapping the code inside a conditional expression.
    raise Tmdb::Error, parsed_exception_rs['status_message']
  else
    raise Tmdb::Error, e.response
  end
end

def parse_element(ele)
  if ele.is_a? Nokogiri::XML::Text
  ^^ Style/GuardClause: Use a guard clause (`return "#{ele.text}\n" if ele.is_a? Nokogiri::XML::Text`) instead of wrapping the code inside a conditional expression.
    return "#{ele.text}\n"
  else
    wrap_node(ele, ele.text)
  end
end

def parent(indices)
  if indices.empty?
  ^^ Style/GuardClause: Use a guard clause (`raise IndexError, 'cannot get parent of an empty path' if indices.empty?`) instead of wrapping the code inside a conditional expression.
    raise IndexError, 'cannot get parent of an empty path'
  else
    NodePath.new(indices[0...-1])
  end
end

def sibling(indices, offset)
  if indices.empty?
  ^^ Style/GuardClause: Use a guard clause (`raise IndexError, 'cannot get sibling of an empty path' if indices.empty?`) instead of wrapping the code inside a conditional expression.
    raise IndexError, 'cannot get sibling of an empty path'
  else
    *xs, x = indices
    NodePath.new(xs + [x + offset])
  end
end

def with_retries(retriable, retries)
  yield
rescue => e
  if retriable && retries < self.max_retries
  ^^ Style/GuardClause: Use a guard clause (`raise e unless retriable && retries < self.max_retries`) instead of wrapping the code inside a conditional expression.
    retry
  else
    raise e
  end
end

def handle_response(response)
  if response.code.to_i == 200
  ^^ Style/GuardClause: Use a guard clause (`raise HttpServerError.build(response.code, response.body) unless response.code.to_i == 200`) instead of wrapping the code inside a conditional expression.
    Postmark::Json.decode(response.body)
  else
    raise HttpServerError.build(response.code, response.body)
  end
end

# if-else at end of method where else branch is guard and inline is too long
def read_definitions_file
  if ::File.exist?(definitions_file_path)
  ^^ Style/GuardClause: Use a guard clause (`unless ::File.exist?(definitions_file_path); raise LoadError, "Could not find definitions.yml file! Please run the install generator"; end`) instead of wrapping the code inside a conditional expression.
    ::YAML.safe_load_file(definitions_file_path) || []
  else
    raise LoadError, "Could not find definitions.yml file! Please run the install generator"
  end
end

# if-else at end of method where if branch is guard and inline is too long
def can_handle_observation_request?(observation_request, super_only: false)
  observation_request = observation_request.to_s
  super_result = super(observation_request)
  if observation_request.start_with?('on_') && !super_result && !super_only
  ^^ Style/GuardClause: Use a guard clause (`if observation_request.start_with?('on_') && !super_result && !super_only; return menu_item_proxy.can_handle_observation_request?(observation_request); end`) instead of wrapping the code inside a conditional expression.
    return menu_item_proxy.can_handle_observation_request?(observation_request)
  else
    super_result
  end
end

# Nested bare if at end of if-branch (recursion into ending body)
def test_nested_ending_if
  if outer_condition
  ^^ Style/GuardClause: Use a guard clause (`return unless outer_condition`) instead of wrapping the code inside a conditional expression.
    other_work
    if inner_condition
    ^^ Style/GuardClause: Use a guard clause (`return unless inner_condition`) instead of wrapping the code inside a conditional expression.
      nested_work
    end
  end
end

# Nested bare unless at end of unless-branch (recursion into ending body)
def test_nested_ending_unless
  unless outer_condition
  ^^^^^^ Style/GuardClause: Use a guard clause (`return if outer_condition`) instead of wrapping the code inside a conditional expression.
    other_work
    unless inner_condition
    ^^^^^^ Style/GuardClause: Use a guard clause (`return if inner_condition`) instead of wrapping the code inside a conditional expression.
      nested_work
    end
  end
end

# Unparenthesized assignment in condition remains an offense
def test_unparenthesized_assignment
  if record = call_recorder.record
  ^^ Style/GuardClause: Use a guard clause (`return unless record = call_recorder.record`) instead of wrapping the code inside a conditional expression.
    @collector.handle_record(record)
  end
end

# Parenthesized assignment is only accepted when the branch uses the local in a descendant node
def test_parenthesized_assignment_plain_read
  if (foo = bar)
  ^^ Style/GuardClause: Use a guard clause (`return unless (foo = bar)`) instead of wrapping the code inside a conditional expression.
    foo
  end
end

# Bare if at end of define_method block body
define_method(:test_method) do
  if enable_demos_tf? && !enable_plugins?
  ^^ Style/GuardClause: Use a guard clause (`return unless enable_demos_tf? && !enable_plugins?`) instead of wrapping the code inside a conditional expression.
    self.enable_plugins = true
  end
end

# Bare if at end of define_method block body with preceding code
define_method(:test_method) do
  install_hooks_method.bind(self).()
  if Pod::is_prebuild_stage
  ^^ Style/GuardClause: Use a guard clause (`return unless Pod::is_prebuild_stage`) instead of wrapping the code inside a conditional expression.
    self.prebuild_frameworks!
  end
end

# Nested bare ifs at end of define_method block body (3 offenses from recursion)
define_method(:test_method) do |integration_name|
  if enabled
  ^^ Style/GuardClause: Use a guard clause (`return unless enabled`) instead of wrapping the code inside a conditional expression.
    registered_integration = Registry.lookup(integration_name)
    if registered_integration
    ^^ Style/GuardClause: Use a guard clause (`return unless registered_integration`) instead of wrapping the code inside a conditional expression.
      klass = registered_integration.klass
      if klass.loaded? && klass.compatible?
      ^^ Style/GuardClause: Use a guard clause (`return unless klass.loaded? && klass.compatible?`) instead of wrapping the code inside a conditional expression.
        instance = klass.new
        instance.patcher.patch unless instance.patcher.patched?
      end
    end
  end
end

# if-else with multi-line if-branch raise, single-line else-branch raise
def test_multiline_guard_fallthrough_raise
  if err.message.include?('not found')
  ^^ Style/GuardClause: Use a guard clause (`raise err unless err.message.include?('not found')`) instead of wrapping the code inside a conditional expression.
    raise parser.error(
      "not found in table"
    )
  else
    raise err
  end
end

# if-else with multi-line if-branch raise, single-line else-branch return
def test_multiline_guard_fallthrough_return
  if raise_if_missing
  ^^ Style/GuardClause: Use a guard clause (`return nil unless raise_if_missing`) instead of wrapping the code inside a conditional expression.
    raise Informative, "Trying to access" \
      " a specification"
  else
    return nil
  end
end

# if-else with multi-line guard inside unless (inner if is the offense)
def test_multiline_guard_nested
  other_work
  unless subspec
    if raise_if_missing
    ^^ Style/GuardClause: Use a guard clause (`return nil unless raise_if_missing`) instead of wrapping the code inside a conditional expression.
      raise Informative, "Unable to find" \
        " a specification"
    else
      return nil
    end
  end
  subspec.do_something
end

def find_template_for(path)
  template or if block_given? then yield
              ^^ Style/GuardClause: Use a guard clause (`raise "No template found for resource #{path}" unless block_given?`) instead of wrapping the code inside a conditional expression.
              else raise "No template found for resource #{path}"
              end
end

def check_record(key, account)
  unless zip.exists?("data/active_storage_blobs/#{key}.json") || ActiveStorage::Blob.exists?(key: key, account: account)
  ^^^^^^ Style/GuardClause: Use a guard clause (`if zip.exists?("data/active_storage_blobs/#{key}.json") || ActiveStorage::Blob.exists?(key: key, account: account); return; end`) instead of wrapping the code inside a conditional expression.
    # File exists without corresponding blob record - could be orphaned or blob not yet imported
    # We allow this since blob metadata is imported before files
  end
end

def render v,*a,&b;if t=lookup(v);r=@_r;@_r=o=Hash===a[-1]?a.pop: {};s=(t==true)?mab{
                   ^^ Style/GuardClause: Use a guard clause (`raise "no template: #{v}" unless t=lookup(v)`) instead of wrapping the code inside a conditional expression.
  send v,*a,&b}: t.render(self,o[:locals]||{},&b);s=render(L,o.merge(L=>false)){s
} if o[L] or o[L].nil?&&lookup(L)&&!r&&v.to_s[0]!=?_;s else raise "no template: #{v}"
end end

(1...key).inject(self.first) { |fun| if fun then self.next(fun) else break end }
                                     ^^ Style/GuardClause: Use a guard clause (`break unless fun`) instead of wrapping the code inside a conditional expression.

(1...key).inject(self.first) { |global| if global then self.next(global) else break end }
                                        ^^ Style/GuardClause: Use a guard clause (`break unless global`) instead of wrapping the code inside a conditional expression.

def call_with_error_handler
  yield(if f.empty? || f.find{ |ff| ff.kind_of?(Exception) } || !h
        ^^ Style/GuardClause: Use a guard clause (`fail(res, h.call(res)) unless f.empty? || f.find{ |ff| ff.kind_of?(Exception) } || !h`) instead of wrapping the code inside a conditional expression.
          res
        else
          fail(res, h.call(res))
        end)
end

def initialize(markup)
  if /(?<fiddle>\w+\/?\d?)(?:\s+(?<sequence>[\w,]+))?(?:\s+(?<skin>\w+))?(?:\s+(?<height>\w+))?(?:\s+(?<width>\w+))?/ =~ markup
  ^^ Style/GuardClause: Use a guard clause (`unless /(?<fiddle>\w+\/?\d?)(?:\s+(?<sequence>[\w,]+))?(?:\s+(?<skin>\w+))?(?:\s+(?<height>\w+))?(?:\s+(?<width>\w+))?/ =~ markup; return; end`) instead of wrapping the code inside a conditional expression.
    @fiddle   = fiddle
    @sequence = (sequence unless sequence == 'default') || 'js,resources,html,css,result'
    @skin     = (skin unless skin == 'default') || 'light'
    @width    = width || '100%'
    @height   = height || '300px'
  end
end
