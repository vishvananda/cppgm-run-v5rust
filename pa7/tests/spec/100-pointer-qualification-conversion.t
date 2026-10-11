// N3485 focus: 4.4 [conv.qual] qualification conversions
// HHC-184
bool f(const char16_t* a, const char16_t* b, const char16_t* p) {
  return a == b && p == a;
}

bool g(const char16_t* a, const char16_t* b, char16_t* p) {
  return f(a, b, p);
}

int main() { return 0; }
