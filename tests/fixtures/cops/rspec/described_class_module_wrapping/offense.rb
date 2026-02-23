module MyModule
^^^^^^^^^^^^^^^ RSpec/DescribedClassModuleWrapping: Avoid opening modules and defining specs within them.
  RSpec.describe MyClass do
    subject { "MyClass" }
  end
end

module MyFirstModule
^^^^^^^^^^^^^^^^^^^^ RSpec/DescribedClassModuleWrapping: Avoid opening modules and defining specs within them.
  module MySecondModule
  ^^^^^^^^^^^^^^^^^^^^^ RSpec/DescribedClassModuleWrapping: Avoid opening modules and defining specs within them.
    RSpec.describe MyClass do
      subject { "MyClass" }
    end
  end
end
