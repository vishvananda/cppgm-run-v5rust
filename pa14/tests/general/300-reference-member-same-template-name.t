struct nil {};

template<class T, class U>
struct cons
{
  typedef T car_type;
  T car;
  U cdr;
};

template<class Cons>
struct iter
{
  Cons & cons;
  iter(Cons & value) : cons(value) {}
};

template<class Cons>
typename Cons::car_type & deref(iter<Cons> const & i)
{
  return i.cons.car;
}

int main()
{
  cons<short, nil> value;
  value.car = 2;
  iter<cons<short, nil> > it(value);
  deref(it) = 7;
  return value.car == 7 ? 0 : 1;
}
