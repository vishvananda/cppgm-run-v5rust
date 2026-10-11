// VALIDATION: compile-pass
// N3485 focus: 5.3.5 [expr.delete]
// A virtual deleting destructor selects the sole class-specific sized
// deallocation function and passes the complete object size.

struct object {
  virtual ~object() {}
  static void operator delete(void *, decltype(sizeof(0))) noexcept;
};

int main() { delete (object *)0; }
