# ... forwarding: both *rest and **kwrest present
def foo(*args, **kwargs, &block)
        ^^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  bar(*args, **kwargs, &block)
      ^^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

def test(*args, **opts, &blk)
         ^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  other(*args, **opts, &blk)
        ^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

def forward_triple_to_super(*args, **opts, &block)
                            ^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  super(*args, **opts, &block)
        ^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

# ... forwarding: both *rest and **kwrest with leading positional param
def method_missing(m, *args, **kwargs, &block)
                      ^^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  @tpl.send(m, *args, **kwargs, &block)
               ^^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

# Anonymous block forwarding (&block -> &) — block only
def run_task(&block)
             ^^^^^^ Style/ArgumentsForwarding: Use anonymous block arguments forwarding (`&`).
  executor.post(&block)
                ^^^^^^ Style/ArgumentsForwarding: Use anonymous block arguments forwarding (`&`).
end

# Anonymous block forwarding with extra positional args
def handle(name, &block)
                 ^^^^^^ Style/ArgumentsForwarding: Use anonymous block arguments forwarding (`&`).
  registry.call(name, &block)
                      ^^^^^^ Style/ArgumentsForwarding: Use anonymous block arguments forwarding (`&`).
end

# Anonymous rest + block forwarding with extra positional args
def dispatch(x, *args, &block)
                ^^^^^ Style/ArgumentsForwarding: Use anonymous positional arguments forwarding (`*`).
                       ^^^^^^ Style/ArgumentsForwarding: Use anonymous block arguments forwarding (`&`).
  handler.run(x, *args, &block)
                 ^^^^^ Style/ArgumentsForwarding: Use anonymous positional arguments forwarding (`*`).
                        ^^^^^^ Style/ArgumentsForwarding: Use anonymous block arguments forwarding (`&`).
end

# ... forwarding with leading args in call site (both *rest and **kwrest)
def before_action(*args, **opts, &block)
                  ^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  set_callback(:action, :before, *args, **opts, &block)
                                 ^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

# Anonymous forwarding with yield
def foo_yield(*args)
              ^^^^^ Style/ArgumentsForwarding: Use anonymous positional arguments forwarding (`*`).
  yield(*args)
        ^^^^^ Style/ArgumentsForwarding: Use anonymous positional arguments forwarding (`*`).
end

# Anonymous kwargs forwarding with yield
def bar_yield(**kwargs)
              ^^^^^^^^ Style/ArgumentsForwarding: Use anonymous keyword arguments forwarding (`**`).
  yield(**kwargs)
        ^^^^^^^^ Style/ArgumentsForwarding: Use anonymous keyword arguments forwarding (`**`).
end

# Anonymous kwrest forwarding with keyword param and explicit hash
def create_msg(token, allowed_mentions: {}, **options)
                                            ^^^^^^^^^ Style/ArgumentsForwarding: Use anonymous keyword arguments forwarding (`**`).
  post(token, { allowed_mentions: allowed_mentions, **options })
                                                    ^^^^^^^^^ Style/ArgumentsForwarding: Use anonymous keyword arguments forwarding (`**`).
end

# Anonymous *, **, & forwarding to ... with extra positional args
def cache_html(template, key, *, **, &)
                              ^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  html(template, *, **, &)
                 ^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

# Anonymous *, **, & forwarding to ... without extra positional args
def to_html(*, **, &)
            ^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  Papercraft.html(self, *, **, &)
                        ^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end
