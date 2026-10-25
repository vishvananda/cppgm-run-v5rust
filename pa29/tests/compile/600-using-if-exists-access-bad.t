struct base { private: static int value; };
struct derived : base {
  using base::value __attribute__((using_if_exists));
};
