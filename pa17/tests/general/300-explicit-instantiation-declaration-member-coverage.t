// An explicit instantiation declaration promises another translation unit
// defines the members, but two kinds of member are not among them: a member of
// a nested class, which the enclosing instantiation reaches, and an implicitly
// declared member, which the class does not write at all.
struct Tracked { ~Tracked(); };

template<class T>
struct Outer
{
  struct Inner
  {
    explicit Inner(int value);
    int held;
  };

  int ordinary() const;
  Tracked tracked;              // makes the implicit destructor non-trivial
};

template<class T>
Outer<T>::Inner::Inner(int value) : held(value) {}

template<class T>
int Outer<T>::ordinary() const { return 1; }

extern template struct Outer<char>;

int main()
{
  Outer<char> outer;
  typename Outer<char>::Inner inner(3);
  return outer.ordinary() + inner.held;
}
