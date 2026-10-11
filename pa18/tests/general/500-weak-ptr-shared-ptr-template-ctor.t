// VALIDATION: compile-pass

template<class T>
struct deleter
{
};

template<class T, class A, class D>
struct shared_ptr
{
private:
  template<class U, class A2, class D2>
  friend struct weak_ptr;

  int pn;

public:
  shared_ptr()
    : pn(0)
  {
  }

  template<class Y>
  shared_ptr(const shared_ptr<Y, A, D> &)
    : pn(0)
  {
  }
};

template<class T, class A, class D>
struct weak_ptr
{
private:
  int pn;

public:
  weak_ptr()
    : pn(0)
  {
  }

  template<class Y>
  weak_ptr(const weak_ptr<Y, A, D> &)
    : pn(0)
  {
  }

  template<class Y>
  weak_ptr(const shared_ptr<Y, A, D> & r)
    : pn(r.pn)
  {
  }
};

template<class T, class A, class D, class U>
shared_ptr<T, A, D> dynamic_pointer_cast(const shared_ptr<U, A, D> &)
{
  return shared_ptr<T, A, D>();
}

struct X
{
};

struct Y : X
{
};

int main()
{
  shared_ptr<X, int, deleter<Y> > source;
  weak_ptr<Y, int, deleter<Y> > target = dynamic_pointer_cast<Y>(source);
  return 0;
}
