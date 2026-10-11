struct X {
  int a;
  int b;
};

struct Holder {
  X storage;
  const X& by_ref() const;
};

const X& Holder::by_ref() const { return storage; }
X by_value(const X& x) { return x; }

int f(Holder& h, const X& in, bool cond) {
  const X local = cond ? h.by_ref() : by_value(in);
  return local.a + local.b;
}
