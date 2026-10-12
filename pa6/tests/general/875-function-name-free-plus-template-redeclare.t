// Declaration name identity, callable facts and exception specification ownership.
template<class T>int operator+(T,T)noexcept(sizeof(T)==4);template<class U>int operator+(U a,U b)noexcept(sizeof(U)==4){return 1;}
