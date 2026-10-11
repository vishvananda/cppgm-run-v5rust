template<class T> struct Base;
using Alias = Base<int>;

template<class T>
struct Base {
  typedef int value_type;
};

Alias::value_type f() { return 7; }

int main() {
  return f() - 7;
}
