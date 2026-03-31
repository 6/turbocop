# Properly aligned string concatenation
msg = "foo" \
      "bar"

text = 'hello' \
       'world'

result = "one" \
         "two"

simple = "no continuation"

# Backslash inside heredoc should not be flagged
x = <<~SQL
  SELECT * FROM users \
  WHERE id = 1
SQL

y = <<~SHELL
  echo "hello" \
    "world"
SHELL

# Properly indented in def body (always_indented context)
def some_method
  'x' \
    'y' \
    'z'
end

# Properly indented in block body (always_indented context)
foo do
  "hello" \
    "world"
end

# Aligned inside method call argument (non-indented context)
describe "should not send a notification if a notification service is not" \
         "configured" do
end

# Aligned in case/when branch (when is NOT always-indented parent)
def message
  case style
  when :first_column_after_left_parenthesis
    'Indent the right bracket the same as the first position ' \
    'after the preceding left parenthesis.'
  end
end

# Properly indented in lambda body (always_indented context, like block)
x = ->(obj) {
  "line one" \
    "line two" \
    "line three"
}

# Aligned operator assignment inside def (not always-indented)
def update_record
  msg = "initial"
  msg +=
    "first part " \
    "second part"
end

# Aligned index operator write inside def
def process_errors
  errors[:detail] +=
    "a valid item " \
    "description"
end

# Properly indented dstr inside if branch within a + concatenation in a def
# (previously a false positive — StatementsNode pass-through bug)
def library_name
  "update " +
    if items.one?
      "#{items.first} requirement " \
        "#{old_version}" \
        "to #{new_version}"
    else
      names = items.map(&:name).uniq
      if names.one?
        "requirements for #{names.first}"
      else
        "#{names[0..-2].join(', ')} and #{names[-1]}"
      end
    end
end

# Indented dstr inside if/else branch assigned with += (always-indented: parent is :if)
def build_intro
  msg = "Updates dependency"
  msg += if items.count > 2
           " #{links[0..-2].join(', ')} " \
             "and #{links[-1]}. "
         else
           " ancestor dependency #{links[1]}. "
         end
  msg
end

# Indented dstr inside elsif branch (if is always-indented parent)
def application_name
  "bump " +
    if items.one?
      record = items.first
      "#{record.name} " \
        "#{from_msg(record.version)}" \
        "to #{record.new_version}"
    elsif updating_property?
      record = items.first
      "#{prop_name} " \
        "#{from_msg(record.version)}" \
        "to #{record.new_version}"
    else
      names = items.map(&:name).uniq
      names.first
    end
end

# Indented dstr inside block within if branch
def metadata_info
  items.map do |dep|
    msg = if dep.removed?
            "\nRemoves `#{dep.name}`\n"
          else
            "\nUpdates `#{dep.name}` " \
              "#{from_msg(dep.version)}" \
              "to #{dep.new_version}"
          end
    msg
  end
end

# Indented dstr inside conditional assignment after if/else
def render_message
  msg = ""
  msg +=
    if records.count > 10
      "- Additional items viewable in " \
        "[compare view](#{url})\n"
    else
      "- See full diff in [compare view](#{url})\n"
    end
  msg
end

# Aligned dstr inside case/when branch (when is NOT always-indented parent)
def check_visibility
  case role
  when 'default'
    user_ids = 'test'
    "(private = false " \
      "OR author = user " \
      "OR assigned_to IN (ids))"
  when 'own'
    "(author = user OR " \
    "assigned_to IN (ids))"
  end
end

# Aligned dstr inside else of case (case else is NOT always-indented parent)
def bracket_message
  case style
  when :first_column
    'Indent the right bracket the same as the first position ' \
    'after the preceding left parenthesis.'
  else
    'Indent the right bracket the same as the start of the line ' \
    'where the left bracket is.'
  end
end

# Aligned dstr inside else of if within a def body
def enqueue_message
  if success
    "Enqueued all jobs"
  else
    "Failed enqueuing jobs " \
      "to adapter"
  end
end

# Aligned dstr inside rescue clause (rescue is NOT always-indented parent)
def inspect_value
  obj.inspect
rescue StandardError
  "<span class='error'>(Object too large. " \
  "Adjust the maximum size.)</span>"
end

# Indented dstr inside a multi-statement explicit begin/rescue body
def price_tooltip(collectable, data = nil)
  begin
    price = data || @prices[collectable.item_id]

    "<b>#{t('prices.price')}:</b> #{number_with_delimiter(price['price'])} Gil<br>" \
      "<b>#{t('prices.world')}:</b> #{price['world']}<br>" \
      "<b>#{t('prices.updated')}:</b> #{price['last_updated']}"
  rescue
  end
end

# Indented dstr inside else of if (if is always-indented parent)
# Multi-statement if body with block, single-statement else body
def update_message(items, version, adapter)
  if items.any?
    messages = items.map do |dep|
      "  #{dep['explanation']}"
    end.join("\n")

    pluralized =
      items.count > 1 ? "dependencies" : "dependency"

    "The latest possible version that can be installed is " \
      "#{version} because of the following " \
      "conflicting #{pluralized}:\n\n#{messages}"
  else
    "The latest possible version of #{adapter} that can " \
      "be installed is #{version}"
  end
end
