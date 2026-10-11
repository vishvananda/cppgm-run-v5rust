namespace std {
class type_info {
public:
  bool operator==(const type_info&) const;
  bool operator!=(const type_info&) const;
};
}

struct Foo {
  int value;
  Foo(int v) : value(v) {}
};

template<class T>
int same_cloned_type(const T &r)
{
  T *copy = new T(r);
  int same = typeid(r) == typeid(*copy);
  delete copy;
  return same;
}

int main()
{
  Foo f(7);
  return same_cloned_type(f) ? 0 : 1;
}
