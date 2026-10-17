// expect exit 0
// N3485: 5.2.9p2,p11 and 8.5.3p5. References and pointers retain the
// selected base subobject, including after a cast result is bound again.
struct Head {
  int head;
  constexpr Head(int n) : head(n) {}
};
struct Base {
  int value;
  constexpr Base(int n) : value(n) {}
};
struct Derived : Head, Base {
  int tail;
  constexpr Derived(int n) : Head(n), Base(n + 1), tail(n + 2) {}
};
constexpr const Derived& down(const Base& b) {
  return static_cast<const Derived&>(b);
}
constexpr int read(int n) {
  Derived d(n);
  const Base* b = &d;
  return down(*b).tail + static_cast<const Base&>(Derived(n)).value;
}
constexpr Derived object(5);
constexpr const Base* pointer = &object;
constexpr const Base& reference = object;
constexpr Base sliced = object;
static_assert(read(5) == 13, "invocation storage and returned cast route");
static_assert(read(9) == 21, "a repeated call uses its own object");
static_assert(pointer->value == 6, "published pointer base offset");
static_assert(reference.value == 6, "published reference base offset");
static_assert(sliced.value == 6, "copy initialization consumes its selected binding");
struct Source {
  constexpr operator const Derived*() const { return &object; }
};
constexpr const Base* converted = Source();
static_assert(converted->value == 6, "conversion result has its own base route");
typedef const Base* BasePointer;
typedef const Derived* DerivedPointer;
constexpr int alias_read(const Base* p) { return DerivedPointer(p)->tail; }
static_assert(alias_read(BasePointer(&object)) == 7,
              "functional pointer casts share the recorded cast routes");
struct Outer : Head, Derived {
  constexpr Outer() : Head(9), Derived(5) {}
};
constexpr int outer_read() {
  Outer object;
  return static_cast<const Derived*>(static_cast<const Base*>(&object))->tail;
}
static_assert(outer_read() == 7,
              "direct base initialization and an embedded base projection");
int main() {
  return pointer->value == 6 && reference.value == 6 && converted->value == 6
      ? 0 : 1;
}
