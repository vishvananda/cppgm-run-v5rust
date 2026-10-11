template<class T> struct Wrap { T val; };
struct Holder { Wrap<int> w; int extra; };

int main() {
  Holder h;
  h.w.val = 99;
  h.extra = 1;
  return h.w.val - 99;
}
