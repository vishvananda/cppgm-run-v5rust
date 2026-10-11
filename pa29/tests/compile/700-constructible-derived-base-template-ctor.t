struct X {};

template<typename T>
struct Base {};

template<typename T>
struct Derived : Base<T> {};

template<typename T>
struct Target {
  template<typename U>
  Target(Base<U>&&) {}
};

static_assert(__is_constructible(Target<const X>, Derived<X>),
              "constructor template deduction can use a derived template base");

int main()
{
  return 0;
}
