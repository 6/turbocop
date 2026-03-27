if x > 1
  do_something
end

while x > 1
  do_something
end

until x > 1
  do_something
end

if x
  bar
end

while running
  process
end

do_something unless condition

result = foo if bar

run_task until done

def make_admin_if_requested(obj, json)
  begin
    return if json.is_admin === obj.can?(:administer_system)
  rescue PermissionNotFound
  end
end

def inverse
  self.each_pair { |k, v|
    if Array === v
      v
    else
      nil
    end
  }
end

def image_entries(cur_manifest)
  cur_manifest.entries.each do |entry|
    if entry[:entry_type] === :image
      entry.cacheable_url
    end
  end
end
