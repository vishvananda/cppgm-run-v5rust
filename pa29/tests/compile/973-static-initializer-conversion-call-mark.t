struct Temporary {
  Temporary();
  ~Temporary() noexcept(false);
  operator int();
};
struct Value { Value(int); ~Value(); };
void selected() { static Value value(Temporary{}); }
