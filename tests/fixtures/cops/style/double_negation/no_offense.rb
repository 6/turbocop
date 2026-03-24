!something

x = !foo

y = true

z = false

result = condition ? true : false

# not not is not flagged
not not something

# allowed_in_returns (default): !! at end of method body is OK
def active?
  !!@active
end

def valid?
  !!validate
end

def admin?
  !!current_user&.admin
end

# !! as part of a larger expression in return position
def comparison?
  !!simple_comparison(node) || nested_comparison?(node)
end

def allow_if_method_has_argument?(send_node)
  !!cop_config.fetch('AllowMethodsWithArguments', false) && send_node.arguments.any?
end

# !! with explicit return keyword
def foo?
  return !!bar if condition
  baz
  !!qux
end

# !! in if/elsif/else at return position
def foo?
  if condition
    !!foo
  elsif other
    !!bar
  else
    !!baz
  end
end

# !! in if/elsif/else with preceding statements at return position
def bar?
  if condition
    do_something
    !!foo
  elsif other
    do_something
    !!bar
  else
    do_something
    !!baz
  end
end

# !! in unless at return position
def foo?
  unless condition
    !!foo
  end
end

# !! in case/when at return position
def foo?
  case condition
  when :a
    !!foo
  when :b
    !!bar
  else
    !!baz
  end
end

# !! in rescue body at return position
def foo?
  bar
  !!baz.do_something
rescue
  qux
end

# !! in ensure body at return position
def foo?
  bar
  !!baz.do_something
ensure
  qux
end

# !! in rescue + ensure body at return position
def foo?
  bar
  !!baz.do_something
rescue
  qux
ensure
  corge
end

# !! in define_method block
define_method :foo? do
  bar
  !!qux
end

# !! in define_singleton_method block
define_singleton_method :foo? do
  bar
  !!qux
end

# !! with a line-broken expression at return position
def foo?
  return !!bar if condition
  baz
  !!qux &&
    quux
end

# !! on the last line of a multi-line && at last statement (no offense for the last one)
def snapshots_transporter?
  config.snapshots_transport_destination_url &&
  !!config.snapshots_transport_auth_key
end

# !! in XOR expression at last statement
def compare_metadata
  if (!!timekey ^ !!timekey2) || (!!tag ^ !!tag2)
    -1
  else
    0
  end
end

# !! in ternary at last statement
def render_response
  render json: json_obj, status: (!!success) ? 200 : 422
end

# !! as method argument at last statement
def validate_visibility(topic)
  !guardian.can_create_unlisted_topic?(topic, !!opts[:embed_url])
end

# !! in hash value as argument (keyword hash) at last statement
def fetch_topic_view
  render_json_dump(
    TopicViewPostsSerializer.new(
      @topic_view,
      scope: guardian,
      include_raw: !!params[:include_raw],
    ),
  )
end

# !! in array at last statement (array is method arg, not literal return)
def authenticate_with_http(username, password)
  result = user && authenticate(username, password)
  [!!result, username]
end

# !! on same line as last statement in if condition
def clear_capabilities(opts, target_file)
  if !!opts[:clear_capabilities]
    @capng.clear(:caps)
    ret = @capng.apply_caps_file(target_file)
  end
end

# !! in elsif branch at return position (single-stmt elsif body, conditional
# covers def body's last child)
def invite(username, invited_by, guardian)
  if condition_a
    call_one(invited_by, guardian)
  elsif condition_c
    !!generate_record(
      invited_by,
      topic: self,
    )
  end
end

# !! inside hash value in if branch where if is last statement
def configuration_for_custom_finder(finder_name)
  if finder_name.to_s.match(/^find_(all_)?by_(.*?)(!)?$/)
    {
      all: !!$1,
      bang: !!$3,
      fields: $2.split('_and_')
    }
  end
end

# !! in assignment inside block inside conditional at last statement
def root_dir
  existing_paths = root_paths.select { |path| File.exist?(path) }
  if existing_paths.size > 0
    MultiplexedDir.new(existing_paths.map do |path|
      dir = FileSystemEntry.new(name, parent, path)
      dir.write_pretty_json = !!write_pretty_json
      dir
    end)
  end
end

# !! inside hash value in method call args inside respond_to block inside conditional
def show
  if current_user.can?(:show, resource)
    respond_to do |format|
      format.html do
        render Views::Show.new(
          record: @record, export: !!params[:export], bot: browser.bot?
        )
      end
    end
  else
    respond_with_error(403)
  end
end

# !! inside boolean expression at last statement inside if branch
def filter_data(data, transient)
  if (!!data[:transient]) == transient
    defs << {
      name: data[:name],
      automount: !!data[:automount]
    }
  end
end

# !! in method call keyword arg inside conditional branch (multi-line call)
def start_server
  if @extract_enabled && @extract_tag_key
    server_create(:in_tcp, @port, bind: @bind, resolve_name: !!@source_hostname_key) do |data, conn|
      process(data)
    end
  else
    server_create(:in_tcp_batch, @port, bind: @bind, resolve_name: !!@source_hostname_key) do |data, conn|
      process(data)
    end
  end
end

# !! in assignment expression at last statement inside else branch
def process_result
  if block_given?
    result = yield
    actions.each { |action| results[action] = result }
    !!result
  else
    actions.compact.each { |action| results[action] = object.send(action) }
    results.values.all?
  end
end

# FP fix: !! inside assignment (ivar) inside multi-statement conditional branch
# The assignment is NOT a begin_type parent, so the stmts_last_line check should not apply
def lax_parse(markup)
  if markup =~ /syntax/
    @variable_name = match_result(1)
    collection_name = match_result(2)
    @reversed = !!match_result(3)
    @name = "#{@variable_name}-#{collection_name}"
    @collection_name = parse_expression(collection_name)
  else
    raise SyntaxError
  end
end

# FP fix: !! inside local variable assignment inside multi-statement conditional branch
def price_break_down_locals(tx, conversation)
  if tx.payment_process == :none
    nil
  else
    booking = !!tx.booking
    booking_per_hour = tx.booking_per_hour
    quantity = tx.listing_quantity
    show_subtotal = !!tx.booking || quantity.present? && quantity > 1
    TransactionViewUtils.price_break_down_locals({
      booking: booking,
      show_subtotal: show_subtotal,
    })
  end
end

# FP fix: !! inside catch/block wrapper at end of conditional branch
def run_actions
  catch_exceptions do
    @success = if block_given?
                 result = yield
                 actions.each { |action| results[action] = result }
                 !!result
               else
                 actions.compact.each { |action| !skip_actions && (results[action] = object.send(action)) }
                 results.values.all?
               end
  end
end

# FP fix: !! inside hash arg of method call inside if branch where block wraps last_child
def tab_context_menu(tab)
  MenuBuilder.build do
    if tab.is_a?(EditTab)
      path = tab.edit_view.document.path
      item("Copy path", enabled: !!path) { clipboard << path if path }
    end
  end
end

# FP fix: !! inside nested conditional in single-statement body (return position)
# When the if is the ONLY statement, child_nodes.last digs into the if, finding
# the inner if. The inner if covers the branch → return position → no offense.
def invite(username, invited_by, guardian)
  if condition_a
    if condition_b
      !!call_one(invited_by, guardian)
    else
      !!call_two(invited_by, guardian)
    end
  end
end

# FP fix: !! inside hash value in map block body (block is return position)
# The block body's StatementsNode starts before !! line → return position.
def run_actions
  items.map do |item|
    skipped = seen_items[item.name]
    { type: "recipe", name: item.name, skipped: !!skipped }
  end
end

# FP fix: !! assignment inside block (block body is return position)
# alter do ... end is single-statement body; block body covers the !!.
def with_marker(lineno = 1)
  alter do
    @with_marker   = !!lineno
    @marker_lineno = lineno
  end
end

# FP fix: !! assignment inside synchronize block
def run
  @mutex.synchronize do
    return if @ran_once
    result = yield
    @ran_once = !!result
    if !@ran_once && limited?
      @retries += 1
    end
    result
  end
end

# FP fix: !! hash value inside synchronize block
def data_for(thread, purpose, compatible)
  @monitor.synchronize do
    data[thread] = {
      thread: thread,
      sharing: @sharing[thread],
      purpose: purpose,
      waiting: !!@waiting[thread],
      sleeper: @sleeping[thread],
    }
  end
end

# FP fix: !! hash assignment inside filter_map block
def properties_list(properties)
  properties.filter_map do |property|
    values = {}
    values["property"] = property["name"]
    values["required"] = !!property["required"]
    values["default_value"] = property["default"]
    values
  end
end

# FP fix: !! assignment inside map block
def process_items(items)
  items.map do |item|
    item_info = item[item.keys.first]
    item_info["failover_ip"] = !!item_info["has_failover"]
    item_info["active_server_ip"] = item_info["server_ip"]
    item_info
  end
end

# FP fix (round 7): !! inside parentheses used as ternary condition
# In Parser AST, (!!expr) creates a begin node. RuboCop's
# find_parent_not_enumerable returns begin, and begin_type? allows the
# lenient same-line check.
def deliver
  notification = (!!evaluate_option(:silent)) ? notification_class.silent : notification_class
  notification.do_something
  notification.do_something_else
end

# FP fix (round 7): !! inside parentheses used in comparison
def filter_data(data, transient)
  defs = []
  if (!!data[:transient]) == transient
    defs << { name: data[:name] }
  end
end
