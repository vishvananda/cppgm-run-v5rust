// N3485 7.3.3p17, 14.6.2: access through a dependent base waits for replay; expect exit 0
struct Base {
protected:
  typedef int type;
  int value() { return 7; }
  operator int() { return 9; }
};
struct Middle : Base {};
template<class T> struct Derived : T {
  using Base::type;
  using Base::value;
  using Base::operator int;
  type n;
};
int main() {
  Derived<Middle> d;
  d.n = 3;
  return d.value() == 7 && int(d) == 9 && d.n == 3 ? 0 : 1;
}
