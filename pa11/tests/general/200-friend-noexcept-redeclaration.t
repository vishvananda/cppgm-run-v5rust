struct X;

void swap(X&, X&) noexcept;

struct X {
  friend void swap(X&, X&) noexcept;
};

int main() { return 0; }
