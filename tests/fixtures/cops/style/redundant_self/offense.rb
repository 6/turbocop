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

# self. in method parameter default values
def singular_label(alt=self.name)
                       ^^^^ Style/RedundantSelf: Redundant `self` detected.
end

def plural_label(alt=self.name)
                     ^^^^ Style/RedundantSelf: Redundant `self` detected.
end

# keyword param default: step_id is param name, run_step_id is not
def plan_event(event, time = nil, execution_plan_id: self.execution_plan_id, step_id: self.run_step_id, optional: false)
                                                                                      ^^^^ Style/RedundantSelf: Redundant `self` detected.
end

def self.check_port(port_num = self.port)
                               ^^^^ Style/RedundantSelf: Redundant `self` detected.
end

# multiline method chain starting with self
grouping_data = self
                ^^^^ Style/RedundantSelf: Redundant `self` detected.
                .groupings

# self.x before x = ... should be flagged (local not yet in scope)
def scale_marks
  return if self.annotation_categories.nil?
            ^^^^ Style/RedundantSelf: Redundant `self` detected.
  annotation_categories = self.annotation_categories.includes(:annotation_texts)
end
