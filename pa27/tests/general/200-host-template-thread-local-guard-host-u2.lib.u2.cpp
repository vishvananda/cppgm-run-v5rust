// second unit
int counter();
template<class T> struct X { static thread_local int n; };
template<class T> thread_local int X<T>::n = counter();
int other() { return X<int>::n; }
