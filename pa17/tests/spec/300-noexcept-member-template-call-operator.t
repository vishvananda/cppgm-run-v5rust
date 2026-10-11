// VALIDATION: compile-pass
// N3485 focus: 5.3.7 [expr.unary.noexcept], 14.5.2 [temp.mem]

template<class T>
T& as_lvalue() noexcept;

struct identity {
  template<class T>
  int operator()(T&) const noexcept {
    return 0;
  }
};

static_assert(noexcept(as_lvalue<const identity>()(as_lvalue<int>())),
              "member-template call operator should be considered noexcept");

int main()
{
  return 0;
}
