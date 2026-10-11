struct U {
  int value;
  U() : value(0) {}
  U(int x) : value(x) {}
  U(const U&) = default;
};

struct S {
  U u;
  S(int x) : u(x) {}
  S(S&& other)
      : u([](S& s) -> decltype(s.u)&& {
          return static_cast<U&&>(s.u);
        }(other)) {}
};

int main() {
  S a(7);
  S b(static_cast<S&&>(a));
  return b.u.value - 7;
}
