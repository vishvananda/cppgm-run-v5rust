struct X {
  X& operator=(const X&) = delete;
  int f() = delete;
};
