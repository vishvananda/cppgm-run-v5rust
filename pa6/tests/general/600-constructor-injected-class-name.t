// Exact frozen types-mode declaration ownership regression.
struct C{C(int);C *next;void f(){C *p;}};
