template <class T>
T id(const char* __attribute__((__diagnose_if__(1, "x", "warning"))) s, T value);

int main() {
  return 0;
}
