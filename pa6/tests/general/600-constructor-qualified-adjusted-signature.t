// Exact frozen types-mode declaration ownership regression.
struct C{C(const int);};C::C(int p){}
