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

# Redundant begin in ||= assignment with single statement
@current_resource_owner ||= begin
                            ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
  instance_eval(&Doorkeeper.config.authenticate_resource_owner)
end

# Redundant begin in = assignment with single statement
x = begin
    ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
  compute_value
end

# Redundant begin in local variable ||= assignment
value ||= begin
          ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
  calculate
end

# Redundant begin inside a do..end block
items.each do |item|
  begin
  ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
    process(item)
  rescue StandardError => e
    handle(e)
  end
end

# Redundant begin inside a lambda block
Thread.new do
  begin
  ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
    run_task
  rescue => e
    log(e)
  end
end

# Redundant begin inside a block with ensure
run do
  begin
  ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
    perform
  ensure
    cleanup
  end
end
