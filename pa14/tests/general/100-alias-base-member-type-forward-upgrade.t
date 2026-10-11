template<class T> struct Base;
using Alias = Base<int>;

template<class T>
struct Base {
  typedef int value_type;
};

struct Derived : Alias {
  value_type f(value_type x) { return x; }
};

int main() {
  Derived d;
  return d.f(0);
}
