namespace N {
  inline namespace I {
    inline namespace J { struct C; }
  }
}
struct N::C { typedef int word; };
void f() { N::I::J::C::word * p; }

namespace Other { namespace { struct C; } }
struct Other::C { typedef int word; };
void unresolved() { Other::C::word * p; }
