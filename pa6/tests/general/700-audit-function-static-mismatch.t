// Frozen PA6 types-mode scope/declaration ownership regression.
struct C{static int f();};int C::f(){}
