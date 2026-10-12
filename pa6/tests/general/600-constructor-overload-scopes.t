// Exact frozen types-mode declaration ownership regression.
struct C{C(){} C(int p){int x=p;} C(long p){long x=p;}};
