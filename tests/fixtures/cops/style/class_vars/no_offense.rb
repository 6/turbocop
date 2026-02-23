class A
  @test = 10
end

class A
  def test
    @@test
  end
end

class A
  def self.test(name)
    class_variable_get("@@#{name}")
  end
end
