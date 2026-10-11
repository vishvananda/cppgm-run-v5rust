struct owner {
  int member;
  void parse_this() { auto f = [this] { return member; }; }
};

template<class... Args>
void parse_captures(Args... args) {
  int x = 0;
  int y = 0;
  auto empty = [] {};
  auto ref_default = [&] { return x; };
  auto copy_default = [=] { return x; };
  auto ref = [&x] { return x; };
  auto copy = [x] { return x; };
  auto mixed = [x, &y] { return x + y; };
  auto copy_mixed = [=, &x] { return x + y; };
  auto ref_mixed = [&, y] { return x + y; };
  auto pack = [args...] {};
  auto default_pack = [&, args...] {};
}
