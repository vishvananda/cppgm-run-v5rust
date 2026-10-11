// A class with a non-trivial copy or move constructor is passed and returned
// by invisible reference: the parameter, and the caller's result object, is
// the object the copy or move constructor initialized (N3485 5.2.2p4,
// 8.5p15; Itanium C++ ABI 3.1.2.3).
struct base { int n; };

struct derived_copy : base {
  const void* self;
  derived_copy(int v) : self(this) { n = v; }
  derived_copy(const derived_copy& other) : base(other), self(this) {}
};

struct copy_and_trivial_move {
  int n;
  const void* self;
  copy_and_trivial_move(int v) : n(v), self(this) {}
  copy_and_trivial_move(const copy_and_trivial_move& other)
    : n(other.n), self(this) {}
  copy_and_trivial_move(copy_and_trivial_move&&) = default;
};

struct trivial_copy_and_move : base {
  const void* self;
  trivial_copy_and_move(int v) : self(this) { n = v; }
  trivial_copy_and_move(const trivial_copy_and_move&) = default;
  trivial_copy_and_move(trivial_copy_and_move&& other)
    : base(other), self(this) {}
};

int take_derived(derived_copy value) {
  return value.self == &value && value.n == 1 ? 0 : 1;
}

int take_mixed(copy_and_trivial_move value) {
  return value.self == &value && value.n == 2 ? 0 : 2;
}

int take_moved(trivial_copy_and_move value) {
  return value.self == &value && value.n == 3 ? 0 : 4;
}

copy_and_trivial_move make_mixed(int v) {
  copy_and_trivial_move local(v);
  return local;
}

copy_and_trivial_move copy_mixed(const copy_and_trivial_move& source) {
  return source;
}

int main() {
  derived_copy first(1);
  copy_and_trivial_move second(2);
  int (*through_pointer)(copy_and_trivial_move) = take_mixed;
  int status = take_derived(first) | through_pointer(second) |
    take_moved(trivial_copy_and_move(3));
  copy_and_trivial_move made = make_mixed(5);
  if (made.self != &made || made.n != 5) status |= 8;
  copy_and_trivial_move copied = copy_mixed(made);
  if (copied.self != &copied || copied.n != 5) status |= 16;
  return status;
}
