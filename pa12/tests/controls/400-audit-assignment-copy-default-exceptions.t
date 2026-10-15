// A selected parameter copy's defaults affect inferred exceptions; expect exit 0
struct Member {
  int n;
  Member() : n(7) {}
  Member(const Member& m, int = 1) noexcept : n(m.n) {}
  Member(Member&& m, int = 2) noexcept : n(m.n) {}
  Member& operator=(Member m) noexcept { n = m.n; return *this; }
};
struct Holder { Member members[9]; };
struct Derived : Member {};
struct Explicit {
  Member m;
  Explicit& operator=(const Explicit&) noexcept = default;
};
struct Plain {
  int n;
  Plain() : n(3) {}
  Plain(const Plain& p) noexcept : n(p.n) {}
  Plain& operator=(Plain p) noexcept { n = p.n; return *this; }
};
struct PlainHolder { Plain p; };
int main() {
  Holder a, b;
  Derived c, d;
  Explicit e, f;
  PlainHolder g, h;
  if (noexcept(a = b) || noexcept(a = static_cast<Holder&&>(b))) return 1;
  if (noexcept(c = d)) return 2;
  if (!noexcept(e = f) || !noexcept(g = h)) return 3;
  b.members[8].n = 11;
  a = b;
  a = static_cast<Holder&&>(b);
  c = d;
  e = f;
  g = h;
  return a.members[8].n == 11 && c.n == 7 && g.p.n == 3 ? 0 : 4;
}
