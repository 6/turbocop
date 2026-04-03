if x == 1
  :a
elsif x == 2
  :b
elsif x == 3
  :c
end

if foo
  bar
elsif baz
  qux
end

if acting_as.respond_to?(method)
  if responds_locally
    false
  else
    if acting_as_persisted?
      true
    else
      responds_locally ? false : true
    end
  end
else
  false
end

if try_run(<<EOF)
int main() {
   Tcl_Interp *ip;
   ip = Tcl_CreateInterp();
   exit((Tcl_Eval(ip, "set tcl_platform(threaded)") == TCL_OK)? 0: 1);
}
EOF
  tcl_enable_thread = true
elsif try_run(<<EOF)
#include <tcl.h>
static Tcl_ThreadDataKey dataKey;
int main() { exit((Tcl_GetThreadData(&dataKey, 1) == dataKey)? 1: 0); }
EOF
  tcl_enable_thread = true
else
  tcl_enable_thread = false
end
