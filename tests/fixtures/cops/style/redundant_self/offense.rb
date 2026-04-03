def foo
  self.bar
  ^^^^ Style/RedundantSelf: Redundant `self` detected.
end

def test
  self.to_s
  ^^^^ Style/RedundantSelf: Redundant `self` detected.
end

def example
  self.method_name
  ^^^^ Style/RedundantSelf: Redundant `self` detected.
end

class Foo
  def self.name_for_response
    self.name.demodulize
    ^^^^ Style/RedundantSelf: Redundant `self` detected.
  end
end

class Bar
  def allowed(other)
    self.exists?(other)
    ^^^^ Style/RedundantSelf: Redundant `self` detected.
  end
end

class ComboProxy < WidgetProxy
  attr_accessor :tool_item_proxy, :swt_tool_item

  def initialize(*init_args, &block)
    super
    self.tool_item_proxy = WidgetProxy.new("tool_item", parent_proxy, [:separator]) if parent_proxy.swt_widget.is_a?(ToolBar)
    self.swt_tool_item = tool_item_proxy&.swt_widget
  end

  def post_add_content
    if self.tool_item_proxy
       ^^^^ Style/RedundantSelf: Redundant `self` detected.
      self.swt_widget.pack
      ^^^^ Style/RedundantSelf: Redundant `self` detected.
      self.tool_item_proxy.text = "filler"
      ^^^^ Style/RedundantSelf: Redundant `self` detected.
      self.tool_item_proxy.width = swt_widget.size.x
      ^^^^ Style/RedundantSelf: Redundant `self` detected.
      self.tool_item_proxy.control = swt_widget
      ^^^^ Style/RedundantSelf: Redundant `self` detected.
    end
  end
end
