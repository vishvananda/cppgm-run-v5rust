namespace A {
namespace B {
template<class R, class P>
struct duration {
  duration() {}
  duration(int) {}
};

struct nano {};
struct milli {};
typedef duration<long long, nano> nanoseconds;
typedef duration<long long, milli> milliseconds;
}
}

namespace A {
namespace B {
template<class R1, class P1, class R2, class P2>
bool less(const duration<R1, P1>&, const duration<R2, P2>&) {
  return true;
}

template<class R1, class P1, class R2, class P2>
bool greater(const duration<R1, P1>& a, const duration<R2, P2>& b) {
  return less(b, a);
}
}
}

bool f(A::B::nanoseconds elapsed) {
  return A::B::greater(elapsed, A::B::milliseconds(128));
}

int main() {
  return f(A::B::nanoseconds()) ? 0 : 1;
}
