namespace n {
struct A { int value; };
struct B { long value; };
}

template<class T>
struct Holder { typedef T type; };

template<class T>
struct Box { T value; };

template<class T>
struct Use {
  typedef Holder<T> Rep;

  int size() {
    Box<typename Rep::type> box;
    return sizeof(box.value);
  }
};

int main()
{
  Use<n::A> a;
  Use<n::B> b;
  return a.size() == sizeof(n::A) && b.size() == sizeof(n::B) ? 0 : 1;
}
