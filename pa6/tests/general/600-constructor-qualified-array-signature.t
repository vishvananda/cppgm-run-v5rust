// Exact frozen types-mode declaration ownership regression.
struct C{C(int[3]);};C::C(int *p){}
