template<class T> struct P { typedef int word; };
template<> struct P<int> { int word; };
typedef int late;

void syntax() {
  P<int>::word ([](){ late *p; return 1; }());
  P<int>::word ([](){ return [](){ return 1; }(); }());
  P<int>::word ([](){ struct local { int f(){ return sizeof(member); } typedef int member; }; return 1; }());
  int late;
}
