x = 1; y = 2
     ^ Style/Semicolon: Do not use semicolons to terminate expressions.

a = 1; b = 2; c = 3
     ^ Style/Semicolon: Do not use semicolons to terminate expressions.
            ^ Style/Semicolon: Do not use semicolons to terminate expressions.

foo; bar
   ^ Style/Semicolon: Do not use semicolons to terminate expressions.

def guard; log('guard'); !@fail_guard; end
         ^ Style/Semicolon: Do not use semicolons to terminate expressions.
                       ^ Style/Semicolon: Do not use semicolons to terminate expressions.
                                     ^ Style/Semicolon: Do not use semicolons to terminate expressions.

def foo(a) x(1); y(2); z(3); end
               ^ Style/Semicolon: Do not use semicolons to terminate expressions.
                     ^ Style/Semicolon: Do not use semicolons to terminate expressions.
                           ^ Style/Semicolon: Do not use semicolons to terminate expressions.

foo { bar; }
         ^ Style/Semicolon: Do not use semicolons to terminate expressions.

items.each { bar; }
                ^ Style/Semicolon: Do not use semicolons to terminate expressions.

arr.map { baz; }
             ^ Style/Semicolon: Do not use semicolons to terminate expressions.

"#{foo;}"
      ^ Style/Semicolon: Do not use semicolons to terminate expressions.

x = "#{foo;}"
          ^ Style/Semicolon: Do not use semicolons to terminate expressions.

"prefix #{foo;}"
             ^ Style/Semicolon: Do not use semicolons to terminate expressions.

"#{;foo}"
   ^ Style/Semicolon: Do not use semicolons to terminate expressions.

x = "a;b"; y = 2
      ^ Style/Semicolon: Do not use semicolons to terminate expressions.
         ^ Style/Semicolon: Do not use semicolons to terminate expressions.

def prx; end
def r500(*); end
module X
  def self.D(*); end
end

def call e
  k,m,*a=X.D e["PATH_INFO"],e['REQUEST_METHOD'].
  downcase,e;k.new(e,m,prx).service(*a).to_a;rescue;r500(:I,k,m,$!,:env=>e).to_a
            ^ Style/Semicolon: Do not use semicolons to terminate expressions.
                                            ^ Style/Semicolon: Do not use semicolons to terminate expressions.
                                                   ^ Style/Semicolon: Do not use semicolons to terminate expressions.
end

@@parameters = {}
@@aliases = {}
@@arity = {}
@@defaults = {
  parameters: @@parameters.each_with_object({}) { |(k, v), p| p[k] = v.dup; },
                                                                          ^ Style/Semicolon: Do not use semicolons to terminate expressions.
  aliases: @@aliases.dup,
  arity: @@arity.dup
}
