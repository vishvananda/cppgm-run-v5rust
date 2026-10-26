// The converted specification cannot interpret an untyped conversion body.
struct Truth {
  constexpr explicit operator bool() const { return true; }
};
void query() noexcept(Truth{});
