// Exact frozen types-mode declaration ownership regression.
struct C{int& x;C(int& p):x(p){}};
