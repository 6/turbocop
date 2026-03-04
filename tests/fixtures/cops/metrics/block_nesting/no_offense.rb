def foo
  if a
    if b
      if c
        x
      end
    end
  end
end

def bar
  unless a
    y
  end
end

# elsif chains do not increase nesting depth
def action_from_button
  if a
    if b
      if params[:update]
        'update'
      elsif params[:list]
        'list'
      elsif params[:unlist]
        'unlist'
      elsif params[:enable]
        'enable'
      elsif params[:disable]
        'disable'
      elsif params[:copy]
        'copy'
      elsif params[:delete]
        'delete'
      end
    end
  end
end

# Modifier if/unless do not count by default (CountModifierForms: false)
def respond_to_destroy(method)
  if method == :ajax
    if called_from_index_page?
      if items.blank?
        items = get_items(page: current_page - 1) if current_page > 1
        render(:index) && return
      end
    end
  end
end

# Method inside nesting: depth carries through def boundaries
unless guard_condition
  class Base
    def process(arg)
      if check_a
        if check_b
          do_something
        end
      end
    end
  end
end

# Multiple rescue clauses are sibling nesting, not nested within each other
def handle_connections
  while running
    if check_condition
      begin
        do_something
      rescue IOError
        retry
      rescue Errno::EPIPE
        next
      rescue Errno::EBADF
        break
      rescue StandardError
        raise
      end
    end
  end
end

# Ternary if and rescue modifier under the same expression should not stack
# nesting depth (mirrors parser AST sibling traversal in RuboCop).
def cast_time(input)
  if input.is_a?(Array)
    Time.zone.local(*input) rescue nil
  else
    unless input.acts_like?(:time)
      input = input.is_a?(String) ? Time.zone.parse(input) : input.to_time rescue input
    end
    input.in_time_zone rescue nil
  end
end
