def foo
  @foo ||= compute
end
def bar?
  @bar ||= calculate
end
def something
  @something ||= fetch
end
def value
  @value ||= expensive
end

# Not memoization: ||= is not the sole/last statement in method body
def setting(key, options = {})
  @definitions ||= {}
  UserSettings::Setting.new(key, options)
end

def readpartial(size)
  @deadline ||= Process.clock_gettime(Process::CLOCK_MONOTONIC) + @read_deadline
  @socket.read_nonblock(size)
end

def process_url
  @card ||= PreviewCard.new(url: @url)
  attempt_oembed || attempt_opengraph
end
