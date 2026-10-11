class S {
  void f(int) const;
public:
  struct Op {
    S& s;
    Op(S& x) : s(x) {}
    void operator()() { s.f(0); }
  };
};
