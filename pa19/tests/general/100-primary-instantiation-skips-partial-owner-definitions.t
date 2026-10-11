template<class T, class A>
struct box {
  void f();
};

template<class A>
struct box<bool, A> {
  void f();
};

template<class T, class A>
void box<T, A>::f() {}

template<class A>
void box<bool, A>::f() {}

box<int, int> b;

int main() {
  b.f();
  return 0;
}
