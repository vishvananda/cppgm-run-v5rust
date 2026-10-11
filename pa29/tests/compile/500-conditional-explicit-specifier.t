// A conditional explicit-specifier decides whether the constructor takes part
// in copy initialization, and an attribute may sit between it and the rest of
// the declaration, which is how libc++ writes pair's constructors.
struct convertible {
  explicit(false) convertible(int) {}
};

struct guarded {
  explicit(1 == 1) guarded(int) {}
  guarded(long) {}
};

struct spaced {
  explicit(false) __attribute__((__visibility__("hidden"))) spaced(int) {}
};

convertible from_int() { return 7; }

guarded from_long() { return 7L; }

spaced from_attributed() { return 7; }

// Invalid immediate conditions discard the constructor before publication.
struct Missing {};
struct PublicCondition {
  constexpr explicit operator bool() const { return true; }
};
class PrivateCondition {
  constexpr explicit operator bool() const { return true; }
};
struct selected_constructor {
  int choice;
  template<class T>
  explicit(T{}) constexpr selected_constructor(T) : choice(1) {}
  constexpr selected_constructor(...) : choice(2) {}
};
static_assert(selected_constructor(Missing{}).choice == 2, "missing conversion fallback");
static_assert(selected_constructor(PrivateCondition{}).choice == 2, "access fallback");
static_assert(selected_constructor(PublicCondition{}).choice == 1, "valid condition");

int main() {
  selected_constructor missing(Missing{}), private_condition(PrivateCondition{});
  selected_constructor valid(PublicCondition{});
  return missing.choice != 2 || private_condition.choice != 2 || valid.choice != 1;
}
