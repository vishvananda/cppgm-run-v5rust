struct S {
  explicit operator bool() const { return true; }
};

bool f(S s) {
  return static_cast<bool>(s);
}
