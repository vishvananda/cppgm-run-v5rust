template<class T> struct P { typedef int word; };
template<> struct P<int> { int word; };
void syntax() { P<int>::word ([](){ return +; }()); }
