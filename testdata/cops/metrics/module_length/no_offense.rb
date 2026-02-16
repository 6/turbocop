module ShortModule
  def foo
    1
  end
end

module EmptyModule
end

module AnotherShort
  def self.bar
    2
  end

  def self.baz
    3
  end

  def self.qux
    4
  end
end

# Namespace module (body is exactly one class) should not be counted
module Outer
  class Inner
    def a; end
    def b; end
    def c; end
    def d; end
    def e; end
    def f; end
    def g; end
    def h; end
    def i; end
    def j; end
    def k; end
    def l; end
    def m; end
    def n; end
    def o; end
    def p; end
    def q; end
    def r; end
    def s; end
    def t; end
    def u; end
    def v; end
    def w; end
    def x; end
    def y; end
    def z; end
    def aa; end
    def bb; end
    def cc; end
    def dd; end
    def ee; end
    def ff; end
    def gg; end
    def hh; end
    def ii; end
    def jj; end
    def kk; end
    def ll; end
    def mm; end
    def nn; end
    def oo; end
    def pp; end
    def qq; end
    def rr; end
    def ss; end
    def tt; end
    def uu; end
    def vv; end
    def ww; end
    def xx; end
    def yy; end
    def zz; end
    def a1; end
    def a2; end
    def a3; end
    def a4; end
    def a5; end
    def a6; end
    def a7; end
    def a8; end
    def a9; end
    def a10; end
    def a11; end
    def a12; end
    def a13; end
    def a14; end
    def a15; end
    def a16; end
    def a17; end
    def a18; end
    def a19; end
    def a20; end
    def a21; end
    def a22; end
    def a23; end
    def a24; end
    def a25; end
    def a26; end
    def a27; end
    def a28; end
    def a29; end
    def a30; end
    def a31; end
    def a32; end
    def a33; end
    def a34; end
    def a35; end
    def a36; end
    def a37; end
    def a38; end
    def a39; end
    def a40; end
    def a41; end
    def a42; end
    def a43; end
    def a44; end
    def a45; end
    def a46; end
    def a47; end
    def a48; end
    def a49; end
    def a50; end
  end
end

# Namespace module (body is exactly one module) should not be counted
module TopLevel
  module Nested
    def a; end
    def b; end
    def c; end
  end
end
