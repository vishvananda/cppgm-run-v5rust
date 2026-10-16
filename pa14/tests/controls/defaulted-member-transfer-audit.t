// expect exit 0
// Defaulted out-of-class members retain the selected calls and storage paths
// for copy-initialization, allocation, and ordinary virtual destructor reuse.
int destroyed;

template<class T> struct Moved {
  T value;
  Moved() : value(3) {}
  Moved(Moved&&);
};
template<class U> Moved<U>::Moved(Moved&&) = default;

template<class T> struct Copied {
  T value;
  Copied() : value(7) {}
  Copied(const Copied&);
};
template<class U> Copied<U>::Copied(const Copied&) = default;

template<class T> struct Base {
  T value;
  Base() : value(5) {}
  virtual ~Base();
};
template<class U> Base<U>::~Base() = default;
struct Derived : Base<int> {
  ~Derived() { ++destroyed; }
};

int main() {
  Moved<int> first;
  Moved<int> second = static_cast<Moved<int>&&>(first);
  if (second.value != 3) return 1;
  Copied<int> source;
  Copied<int>* allocated = new Copied<int>(source);
  if (allocated->value != 7) return 2;
  {
    Derived object;
    Base<int>* base = &object;
    if (base->value != 5) return 3;
  }
  return destroyed - 1;
}
