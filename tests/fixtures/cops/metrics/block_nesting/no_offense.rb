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
