// N3485 6.5.4p1, 8.5.4p5: range-for over a braced-init-list of class objects
// iterates the initializer_list's const E array; expect exit 0
namespace std {
template<class E> class initializer_list {
  const E* b; unsigned long n;
  initializer_list(const E* p, unsigned long k) : b(p), n(k) {}
public:
  initializer_list() : b(0), n(0) {}
  const E* begin() const { return b; }
  const E* end() const { return b + n; }
};
}
struct V { int n; };
int main() { int s = 0; V a = {1}, b = {2}; for (const V& v : {a, b}) s += v.n; return s == 3 ? 0 : 1; }
