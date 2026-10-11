template<class T>
T&& declval();

template<class F, class A0, class... Args>
auto call(F&&, A0&&, Args&&...)
  -> decltype((declval<A0>().*declval<F>())(declval<Args>()...));

struct Holder {
  int f(int) const;
};

int main() {
  return 0;
}
