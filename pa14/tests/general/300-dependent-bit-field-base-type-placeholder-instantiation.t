template<class A>
struct box {
  using size_type = typename A::size_type;

  struct inner {
    size_type cap : sizeof(size_type) * 8 - 1;
    size_type is_long : 1;
  };

  void f();
};

template<class A>
void box<A>::f() {}

int main() {
  return 0;
}
