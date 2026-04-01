if condition
  true
else
  false
end

unless condition
  false
else
  true
end

if foo == bar
  do_something
else
  false
end

if foo && bar
  true
else
  false
end

if foo || bar
  true
else
  false
end

# Ternary with non-boolean condition (hash lookup, variable, method returning non-boolean)
instance_options[:relationships].following[object.id] ? true : false
Redis::Alfred.set(key, value, nx: true, ex: timeout) ? true : false
app ? true : false

# Ternary with non-boolean method (no ? suffix)
foo.do_something ? true : false

# Regex match operators are not boolean (=~ returns MatchData or nil)
result =~ /^running/ ? true : false
text =~ /pattern/ ? true : false
line !~ /^#/ ? true : false
str =~ /mingw|win32|cygwin/ ? true : false
if text =~ /^\s*$/
  true
else
  false
end

# Spaceship operator does not return boolean
foo <=> bar ? true : false

# Multiple elsif with boolean literal branches - should NOT be flagged
if foo
  true
elsif bar > baz
  true
elsif qux > quux
  true
else
  false
end

# Redundant boolean branches nested directly under a single `elsif`
# should not be flagged because RuboCop skips `if` nodes whose immediate
# parent is an `elsif`.
def to_ruby_internal
  if @payload.nil? or @payload.size == 0
    nil
  elsif @payload.size == 1
    @payload[0] == 1 ? true : false
  else
    @payload.map { |v| v == 1 }
  end
end

def method_missing(method, *args)
  if method.to_s =~ /^([a-z]+)_namespace_names$/
    @@ns_cache ||= {}
    @@ns_cache[$1] ||= get_namespace_names_for($1)
  elsif method.to_s =~ /^([a-z]+)_namespace\?$/
    namespace_type(args.first) == $1.to_sym ? true : false
  else
    super(method, *args)
  end
end

def exists?
  found = false
  lines_count = 0
  return found = lines_count.positive? if resource[:match].nil?

  match_count = count_matches(new_match_regex)
  found = if resource[:ensure] == :present
            if match_count.zero?
              if lines_count.zero? && resource[:append_on_no_match].to_s == "false"
                true
              else
                !(lines_count.zero? && resource[:append_on_no_match].to_s != "false")
              end
            elsif resource[:replace_all_matches_not_matching_line].to_s == "true"
              false
            elsif lines_count.zero?
              resource[:replace].to_s == "false"
            else
              true
            end
          elsif match_count.zero?
            if lines_count.zero?
              false
            else
              true
            end
          elsif lines_count.zero?
            resource[:match_for_absence].to_s == "true"
          else
            false
          end
end

# Multi-elsif chain (2 elsifs) with predicate methods
if !current_version_array.any?
  false
elsif !new_version_array.any?
  true
elsif have_any_matching_version?
  true
else
  false
end

# Multi-elsif chain (3 elsifs)
if template_name.blank?
  false
elsif template_options.empty?
  true
elsif template_options[:only] && template_options[:only].include?(action_name.to_sym)
  true
elsif template_options[:except] && !template_options[:except].include?(action_name.to_sym)
  true
else
  false
end

# Single negation `!` is not considered boolean-returning by RuboCop
# (only `!!` double negation is). These should not be flagged.
if generate && !verify_options
  false
else
  true
end

if record && !record.can_delete?(self)
  false
else
  true
end

if id && !method
  true
else
  false
end

@stored[key] && !@stored[key].empty? ? true : false

membership.nil? || !membership.exists? ? false : true

!ENV["DOCKER"].nil? && !ENV["DOCKER"].empty? ? true : false

uri.is_a?(URI::HTTP) && !uri.host.nil? ? true : false

(index == 0 && !subscribed?(feed)) ? true : false

# elsif with single `!` in condition (not boolean by RuboCop)
if user&.is_moderator?
  true
elsif user && user.id == user_id && !is_moderated?
  true
else
  false
end

if charged_using_account? && using_account_for_user?
  true
elsif migration_enabled? && !merchant_account&.is_managed?
  false
else
  true
end

if section_node.root?
  false
elsif !children.empty?
  true
else
  false
end

if url.try(:empty) || account.try(:empty?)
  raise "not configured"
elsif !url && !account
  false
else
  true
end

# elsif with =~ regex match (not boolean)
if link["data-skip"]
  link.remove_attribute("data-skip")
  true
elsif link["href"].to_s =~ /unsubscribe/i && !options[:unsubscribe_links]
  true
else
  false
end

# Safe navigation calls (&.) may return nil, not boolean — should not be flagged
password&.match?(RULES[name]) ? true : false
@endpoint&.smtp_client&.secure_socket? ? true : false
foo&.bar? ? true : false
obj&.present? ? true : false
if foo&.active?
  true
else
  false
end

# Calls with blocks are `block` nodes in Parser, not `send` — not boolean-returning
if items.any? { |item| item.valid? }
  false
else
  true
end
records.all? { |r| r.persisted? } ? true : false
