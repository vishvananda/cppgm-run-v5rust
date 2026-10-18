// expect exit 0
template<class T, class U> char accept(U);

template<class X>
struct Object {
  X value;

  template<class T>
  auto pointer(T) -> decltype(this) { return this; }

  template<class T>
  auto pointer(T) const -> decltype(this) { return this; }

  template<class T>
  auto read(T) const -> decltype(this->value) { return value; }

  template<class T>
  auto deferred(T) -> decltype(accept<T>(this)) { return T::missing; }
};

int main() {
  Object<int> a = {7};
  const Object<char> b = {3};
  static_assert(sizeof(a.deferred(0)) == 1, "signature does not demand body");
  return a.pointer(0) == &a && b.pointer(0) == &b
      && a.read(0) == 7 && b.read(0) == 3 ? 0 : 1;
}
