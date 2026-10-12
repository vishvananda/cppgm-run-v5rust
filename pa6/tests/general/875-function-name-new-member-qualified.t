// Declaration name identity, callable facts and exception specification ownership.
struct C{void* operator new(unsigned long);};void* C::operator new(unsigned long n){return 0;}
