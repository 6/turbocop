def func
  begin
  ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
    ala
  rescue => e
    bala
  end
end

def Test.func
  begin
  ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
    ala
  rescue => e
    bala
  end
end

def bar
  begin
  ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
    do_something
  ensure
    cleanup
  end
end
