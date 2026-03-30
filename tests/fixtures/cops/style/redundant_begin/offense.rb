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

def nodes_by_class(klass, name)
  @nodes_by_name ||= {}
  @nodes_by_name[name] ||= begin
                           ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
    descendants.select do |e|
      e.kind_of? klass
    end
  end
end

def value(record, field)
  if field.association?
    field.reflection
  else
    begin
    ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
      field.value(record)
    end
  end
end

Thread.new do
  unless @fork_instrumenting
    begin
    ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
      @fork_instrumenting = true
    end
  end
end

def self.parse_binary_dos_format(binary_dos_date, binary_dos_time)
  second = 2 * (0b11111 & binary_dos_time)
  minute = (0b11111100000 & binary_dos_time) >> 5
  hour = (0b1111100000000000 & binary_dos_time) >> 11
  day = (0b11111 & binary_dos_date)
  month = (0b111100000 & binary_dos_date) >> 5
  year = ((0b1111111000000000 & binary_dos_date) >> 9) + 1980
  begin
  ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
    local(year, month, day, hour, minute, second)
  end
end

begin
^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
  Question.first
end

x = 1

begin
  begin 1 end
  ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
end

def join_thread(thr)
  begin thr.join() if thr.alive? rescue nil end
  ^^^^^ Style/RedundantBegin: Redundant `begin` block detected.
end
