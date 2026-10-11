struct impl {
  typedef int color_ref;

  int color() { return 3; }
};

template<class T>
struct trampoline : impl {
};

struct inner : trampoline<long> {
};

struct outer : inner, trampoline<int> {
private:
  typedef trampoline<int> direct_trampoline;

public:
  typedef typename direct_trampoline::color_ref impl_color_ref;

  int color() { return direct_trampoline::color(); }
};

int main() {
  outer::impl_color_ref value = 0;
  outer object;
  impl* first = static_cast<inner*>(&object);
  impl* second = static_cast<trampoline<int>*>(&object);
  if(first == second) return 2;
  return value + object.color() == 3 ? 0 : 1;
}
