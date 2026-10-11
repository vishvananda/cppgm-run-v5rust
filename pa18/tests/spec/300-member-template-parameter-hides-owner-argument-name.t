// N3485 focus: 14.6.4.1 [temp.local], a template-parameter hides an outer name.
template<class T> struct arg {};
template<class T> struct count {
  count() {}
  template<class Y> count(arg<T> const&, count<Y> const&) {}
};
template<class T> struct ptr {
  count<T> value;
  ptr() {}
  template<class Y> ptr(ptr<Y> const& p) : value(arg<T>(), p.value) {}
};
struct X {};
struct Y {};
int main() { ptr<X> x; ptr<Y> y(x); return 0; }
