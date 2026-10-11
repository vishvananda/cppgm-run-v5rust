struct C {
  C(const C&) {}
  C(C&&) noexcept {}
};

static_assert(!__has_nothrow_copy(C), "");
int main() { return 0; }
