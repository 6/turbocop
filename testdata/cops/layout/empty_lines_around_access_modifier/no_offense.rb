class Foo
  def bar
  end

  private

  def baz
  end

  protected

  def qux
  end
end

# Access modifier right after class opening (no blank needed before)
class Bar
  private

  def secret
  end
end

# Access modifier right before end (no blank needed after)
class Baz
  def stuff
  end

  private
end

# Comment before modifier counts as separator
class Qux
  def stuff
  end

  # These methods are private
  private

  def secret
  end
end

# Access modifier as first statement in a block body (no blank needed before)
Class.new do
  private

  def secret
  end
end

# Struct with block
Struct.new("Post", :title) do
  private

  def secret
  end
end

# `private` used as a hash value (not an access modifier)
class Message
  def webhook_data
    {
      message_type: message_type,
      private: private,
      sender: sender
    }
  end

  private

  def secret
  end
end

# `private` used inside a method body conditional (not an access modifier)
class Conversation
  def update_status
    if waiting_present && !private
      clear_waiting
    end
  end

  private

  def clear_waiting
  end
end
