struct V { void clear() {} };

struct O {
  static thread_local V n;

  struct G {
    int f() {
      n.clear();
      return 0;
    }
  };
};

int main() {
  O::G g;
  return g.f();
}
