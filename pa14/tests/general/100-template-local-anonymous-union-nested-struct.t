template<class T>
struct H {
  int f(T v) const {
    union {
      T t;
      struct {
        int a;
        int b;
      } __s;
    } __u;
    __u.t = v;
    return 0;
  }
};

struct P { long a; long b; };

int main() {
  P p = {0, 0};
  H<P> h;
  return h.f(p);
}
