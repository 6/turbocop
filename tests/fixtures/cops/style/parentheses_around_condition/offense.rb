if (x > 1)
   ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `if`.
  do_something
end

while (x > 1)
      ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of a `while`.
  do_something
end

until (x > 1)
      ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `until`.
  do_something
end

if (x)
   ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `if`.
  bar
end

while (running)
      ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of a `while`.
  process
end

do_something unless (condition)
                    ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `unless`.

result = foo if (bar)
                ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `if`.

run_task until (done)
               ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `until`.

def make_admin_if_requested(obj, json)
  begin
    return if (json.is_admin === obj.can?(:administer_system))
              ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `if`.
  rescue PermissionNotFound
  end
end

def inverse
  self.each_pair { |k, v|
    if (Array === v)
       ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `if`.
      v
    else
      nil
    end
  }
end

def image_entries(cur_manifest)
  cur_manifest.entries.each do |entry|
    if (entry[:entry_type] === :image)
       ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `if`.
      entry.cacheable_url
    end
  end
end
