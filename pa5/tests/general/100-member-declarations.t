struct C {
  C();
  ~C();
  operator int() const;
  int operator[](int i) const;
  C& operator=(const C&) = default;
  int value{};
  int method() { return 0; };
  virtual int* pure(int x, const C&& y) = 0;
};
