// VALIDATION: compile-pass
// A demanded out-of-class member-template body is instantiated only after the
// enclosing class is complete, so its implicit copy assignment is available.

template<class T>
struct box
{
  template<class U>
  void assign(U& dst, const U& src);

  void demand(T& dst, const T& src)
  {
    assign(dst, src);
  }
};

template<class T>
template<class U>
void box<T>::assign(U& dst, const U& src)
{
  dst = src;
}

struct node
{
  box<node> member;
  int value;
};

int main()
{
  node dst;
  node src;
  dst.value = 1;
  src.value = 2;
  dst.member.demand(dst, src);
  return dst.value != 2;
}
