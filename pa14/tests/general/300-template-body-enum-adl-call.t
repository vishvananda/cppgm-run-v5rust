namespace N {
namespace C {
enum E { A = 0, B = 1 };
inline E helper(E e) { return e; }
}

template<class T>
struct S {
  typedef C::E flag_type;
  flag_type f;

  bool g() const { return helper(f) == C::A; }
};
}

int main() { return 0; }
