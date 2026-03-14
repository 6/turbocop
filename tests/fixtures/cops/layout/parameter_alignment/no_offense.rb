def foo(bar,
        baz)
  123
end

def method_a(x, y)
  x + y
end

def method_b(a,
             b,
             c)
  a + b + c
end

# Multiple params on a continuation line should not flag later params
def correct(processed_source, node,
            previous_declaration, comments_as_separators)
  processed_source
end

# Parameters after a multi-line default value should not be flagged
def initialize(redis_client: Redis.new(
  url: ENV.fetch("TAXONOMY_CACHE_REDIS_URL", ENV["REDIS_URL"]),
  reconnect_attempts: 4,
  reconnect_delay: 15,
  reconnect_delay_max: 60,
), adapter: PublishingApiAdapter.new)
  @redis_client = redis_client
  @adapter = adapter
end

# Block parameter aligned with first parameter
def foo(bar,
        &blk)
  bar
end
