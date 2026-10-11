template<class T>
struct S {
  template<class U>
  static auto g(T&& t, U&& u)
      -> decltype(static_cast<T&&>(t), static_cast<U&&>(u), int()) {
    return 0;
  }
};

int f() { return 7; }
typedef int (&FnRef)();

int main() {
  return S<FnRef>::g<int>(f, 0);
}
