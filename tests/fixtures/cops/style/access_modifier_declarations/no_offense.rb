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

  # Visibility-change calls (not inline modifier declarations)
  public target
  private method_var
  protected some_method_name
end
