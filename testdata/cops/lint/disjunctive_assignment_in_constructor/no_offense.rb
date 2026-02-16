class Foo
  def initialize
    @x = 1
  end
end

class Bar
  def initialize
    @a = []
    @b = {}
  end
end

class Baz
  def some_method
    @x ||= 1
  end
end

# ivar ||= after a plain assignment should NOT be flagged
# because we can't be certain the disjunction is unnecessary
class Report
  def initialize(type)
    @type = type
    @start_date ||= 30.days.ago
    @end_date ||= Time.now
  end
end

# ivar ||= after super should NOT be flagged
class Derived
  def initialize
    super
    @config ||= 'default'
  end
end

# ivar ||= after any method call should NOT be flagged
class WithMethodCall
  def initialize
    setup_defaults
    @x ||= 1
  end
end

# ivar ||= after a conditional should NOT be flagged
class WithConditional
  def initialize(opts)
    @mutex = Mutex.new
    @dbs ||= Set.new
  end
end
