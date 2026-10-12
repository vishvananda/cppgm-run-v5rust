// Dependent base edges and specialization edges have distinct lookup rules.
template<class T> struct Base { using word = int; };
template<class T> struct Derived : Base<T> { void f() { word * p; } };
template<class T> struct Both { using first = int; };
template<> struct Both<int> { using second = int; };
void f() { Both<float>::first * p; Both<int>::second * q; }

template<class T> struct Mixed : Base<T>, Base<int> { void f() { word * p; } };
