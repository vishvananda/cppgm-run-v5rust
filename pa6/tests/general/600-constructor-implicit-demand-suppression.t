// Exact frozen types-mode declaration ownership regression.
struct C{C(const C&);};void f(){C();}
