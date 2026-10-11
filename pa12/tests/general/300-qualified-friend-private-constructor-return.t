namespace N {
struct Id;
namespace M { Id make(); }

struct Id {
private:
  int* p_;
  Id(int* p) : p_(p) {}
  friend Id M::make();
public:
  int* get() const { return p_; }
};

int g;

namespace M {
Id make() {
  return &g;
}
}
}

int main() {
  return N::M::make().get() == &N::g ? 0 : 1;
}
