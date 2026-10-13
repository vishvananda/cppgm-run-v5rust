int f(volatile void*);int f(const volatile void*);int g(volatile int*p){return f(p);}
