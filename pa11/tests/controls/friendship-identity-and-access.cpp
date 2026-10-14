// expect exit 0
// Friend identities, hidden ADL, signature access and inherited protected access.
class Vault {
  typedef int Key;
  static const int seed = 3;
  int value;
  friend Key inspect(Vault &, Key);
public:
  Vault(int n) : value(n) {}
  friend int read(Vault & v) { return v.value; }
  friend int read(const Vault & v) { return v.value + 1; }
};

Vault::Key inspect(Vault & v, Vault::Key extra = Vault::seed) {
  return v.value + extra;
}

class Reader;
class Base {
protected:
  int value;
public:
  Base(int n) : value(n) {}
};
class Derived : private Base {
  friend class Reader;
public:
  Derived(int n) : Base(n) {}
};
class Reader {
public:
  int read(Derived & d) { return ++d.value; }
};

struct Outer {
  enum Kind { chosen = 5 };
  struct Nested { int value; };
  friend int tag(Kind n) { return static_cast<int>(n); }
  friend int nested(const Nested & n) { return n.value; }
};

struct Box;
namespace detail { void set(Box &); }
struct Box {
  Box() : value(0) {}
  int get() const { return value; }
private:
  int value;
  friend void detail::set(Box &);
};
void detail::set(Box & b) { b.value = 7; }

int main() {
  Vault v(4);
  const Vault & c = v;
  Derived d(8);
  Reader r;
  Outer::Nested n{6};
  Box b;
  detail::set(b);
  return inspect(v) == 7 && read(v) == 4 && read(c) == 5 && r.read(d) == 9
      && tag(Outer::chosen) == 5 && nested(n) == 6 && b.get() == 7 ? 0 : 1;
}
