// N3485 focus: 14.8.2 [temp.deduct], substituted function result types
// A function-template result type must use the template parameter binding even
// when an outer type has the same name.

template<class T>
struct copy_argument {
  typedef T type;
};

template<class L, class R>
struct pair2 {
  pair2(const L&, const R&) {}
};

class A {};

struct actual {};

template<class Base>
struct functor {
  template<class A>
  pair2<functor<Base>, typename copy_argument<const A>::type>
  operator=(const A& value) const
  {
    return pair2<functor<Base>, typename copy_argument<const A>::type>(
        *this,
        value);
  }
};

int main()
{
  functor<int> f;
  actual value;
  (void)(f = value);
  return 0;
}
// VALIDATION: compile-pass
