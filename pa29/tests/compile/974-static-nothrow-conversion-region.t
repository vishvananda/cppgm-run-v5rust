struct Temporary {
  Temporary();
  ~Temporary() noexcept(false);
  operator int() noexcept;
};
struct Value { Value(int); ~Value(); };
void selected() { static Value value(Temporary{}); }
