namespace W {
template<class T>
struct Box {};
}

namespace N {
struct T {};

int f(W::Box<T>);
int g(W::Box<T> *);
}

int use_value(W::Box<N::T> x) {
  return f(x);
}

int use_pointer(W::Box<N::T> * x) {
  return g(x);
}

int main() {
  return 0;
}
