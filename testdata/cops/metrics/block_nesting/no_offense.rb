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
