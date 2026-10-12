// Declaration name identity, callable facts and exception specification ownership.
struct C{void* operator new(unsigned long n){return 0;}void operator delete(void* p){}};
