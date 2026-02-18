class Foo
  private

  def bar
    puts 'bar'
  end

  protected

  def baz
    puts 'baz'
  end

  private :some_method
end
