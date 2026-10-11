// VALIDATION: compile-pass
// Forwarding a const array reference must not deduce const into its element type.
template<class M>
void inner(M const&) {
  M copy = {};
  copy[0][0] = 1;
}

template<class M>
void outer(M const& value) {
  inner(value);
}

int main() {
  int value[2][2] = {};
  outer(value);
}
