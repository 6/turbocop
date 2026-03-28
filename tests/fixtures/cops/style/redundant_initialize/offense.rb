def initialize
^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary empty `initialize` method.
end

def initialize
^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary `initialize` method.
  super
end

def initialize(a, b)
^^^^^^^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary `initialize` method.
  super
end

def initialize(a, b)
^^^^^^^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary `initialize` method.
  super(a, b)
end

def initialize(x)
^^^^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary `initialize` method.
  super(x)
end

def initialize()
^^^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary `initialize` method.
  super()
end

class CommentOnlyMulti1
  include Something

  def initialize
  ^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary empty `initialize` method.
    # comment
  end
end

class CommentOnlyMulti2
  include Foo

  def initialize
  ^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary empty `initialize` method.
    # multi-line
    # comments
  end
end

class InlineCommentMulti
  attr_accessor :child

  def initialize # required
  ^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary empty `initialize` method.
  end
end

class SuperCommentMulti
  include Something

  def initialize
  ^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary `initialize` method.
    super
    # comment
  end
end

def initialize; end if false # dummy for RDoc
^^^^^^^^^^^^^^^^^^^ Style/RedundantInitialize: Remove unnecessary empty `initialize` method.
