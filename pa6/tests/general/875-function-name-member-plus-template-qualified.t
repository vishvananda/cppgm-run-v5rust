// Declaration name identity, callable facts and exception specification ownership.
struct C{template<class T>int operator+(T);};template<class U>int C::operator+(U n){U x=n;return 1;}
