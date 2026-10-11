namespace std { template<class T> class initializer_list {}; }

struct A {
  int n;
  A() : n(1) {}
  A(std::initializer_list<int>) : n(2) {}
};

int main() {
  A a{};
  return a.n == 1 ? 0 : 1;
}
