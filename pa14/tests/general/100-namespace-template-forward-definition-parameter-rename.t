namespace N {
template<class State> struct fpos;
typedef fpos<int> streampos;
}

namespace N {
template<class StateT>
struct fpos {
  StateT st;
  StateT get() { return st; }
};
}

int main() {
  N::streampos p;
  p.st = 5;
  return p.get() - 5;
}
