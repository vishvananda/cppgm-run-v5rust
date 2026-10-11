template<bool B, class T = void>
struct enable_if {
};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

struct Handle {
  Handle & operator=(const Handle &) = delete;

  template<bool Enabled = true>
  typename enable_if<Enabled, Handle &>::type operator=(Handle &&) {
    return *this;
  }
};

static_assert(__is_assignable(Handle &, Handle &&),
              "member assignment templates participate in assignability");

int main() {
  Handle lhs;
  Handle rhs;
  lhs = static_cast<Handle &&>(rhs);
  return 0;
}
