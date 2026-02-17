def foo
  return if need_return?

  bar
end

def baz
  return if something?
  return if something_different?

  bar
end

def quux
  raise "error" unless valid?

  do_work
end

def last_guard
  return if done?
end

def consecutive_with_embedded_return
  return if does_not_expire?
  requeue! && return if not_due_yet?

  notify_remote_voters_and_owner!
end

def consecutive_mixed_guards
  raise "error" unless valid?
  do_something || return if stale?

  process
end

# Comment between consecutive guard clauses is OK
def comment_between_guards
  return if first_condition?
  # This is a comment explaining the next guard
  return if second_condition?

  do_work
end

# Multiple comments between guards
def multi_comment_between_guards
  return unless valid_input?
  # First reason
  # Second reason
  return if already_processed?

  process
end

# Guard followed by multi-line if block containing return
def guard_then_multiline_if
  return if done?
  if complex_condition? && another_check?
    return
  end

  process
end

# Guard followed by multi-line unless block containing raise
def guard_then_multiline_unless
  return unless valid?
  unless authorized? && permitted?
    raise "unauthorized"
  end

  do_work
end

# Guard inside a block (embedded in larger expression)
def guard_in_block
  heredocs.each { |node, r| return node if r.include?(line_number) }
  nil
end

# Break inside a block (embedded in larger expression)
def break_in_block
  prev = items.each_cons(2) { |m, n| break m if n == item }
  between = prev.source_range
end

# Guard before end keyword
def guard_before_end
  return if something?
end

# Guard before else
def guard_before_else(x)
  if x > 0
    return if done?
  else
    work
  end
end

# Guard before rescue
def guard_before_rescue
  return if safe?
rescue => e
  handle(e)
end

# Guard before ensure
def guard_before_ensure
  return if cached?
ensure
  cleanup
end

# Guard before when
def guard_before_when(x)
  case x
  when :a
    return if skip?
  when :b
    work
  end
end

# Next in block followed by comment then next
def next_with_comments
  items.each do |item|
    next if item.blank?
    # Skip already processed items
    next if item.processed?

    process(item)
  end
end

# Raise with comment between guards
def raise_with_comments
  raise "error" unless condition_a?
  # Make sure condition_b is also met
  raise "other error" unless condition_b?

  run
end

# Guard followed by rubocop directive then blank line
def guard_with_directive
  return if need_return?
  # rubocop:disable Metrics/AbcSize

  bar
  # rubocop:enable Metrics/AbcSize
end

# Guard with nocov directive followed by blank line
def guard_with_nocov
  # :nocov:
  return if condition?
  # :nocov:

  bar
end

# Guard clause is last statement before closing brace
def guard_before_closing_brace
  items.map do |item|
    return item if item.valid?
  end
end

# Guard followed by multiline if with return inside nested structure
def guard_then_nested_multiline_if
  return if line_length(line) <= max
  return if allowed_line?(line)
  if complex_config? && special_mode?
    return check_special(line)
  end

  register_offense(line)
end

# Multiple return false guards with comments
def multiple_return_false_guards
  return false unless first_check?
  # anonymous forwarding
  return true if special_case?
  return false unless second_check?
  return false unless third_check?

  name == value
end

# Guard with long comment block between guards
def long_comment_between_guards
  return false unless incoming? || outgoing?
  # For Chatwoot Cloud:
  #   - Enable indexing only if the account is paid.
  #   - The `advanced_search_indexing` feature flag is used only in the cloud.
  #
  # For Self-hosted:
  #   - Adding an extra feature flag here would cause confusion.
  return false if cloud? && !feature_enabled?('search')

  true
end

# Block-form if with guard clause followed by empty line — no offense
def block_guard_with_blank
  if params.blank?
    fail ParamsError, "Missing params"
  end

  process(params)
end

# Block-form if with guard clause at end of method — no offense
def block_guard_at_end
  if invalid?
    raise "invalid"
  end
end

# Block-form if with multiple statements — not a guard clause
def block_not_guard
  if condition?
    setup
    process
  end
  finalize
end
