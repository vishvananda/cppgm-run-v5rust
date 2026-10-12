// Frozen PA6 types-mode scope/declaration ownership regression.
struct C{C(int n)try:x(n){}catch(int e){int y=n;}int x;};
