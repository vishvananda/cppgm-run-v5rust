namespace outer {
namespace chrono {
template<class Rep, class Period>
struct duration {
  typedef Rep rep;
  typedef Period period;
};
}

template<class A, class B>
struct common_type {
  static const int value = 0;
};

template<class R1, class P1, class R2, class P2>
struct common_type<chrono::duration<R1, P1>, chrono::duration<R2, P2> > {
  static const int value = 1;
};
}

int main()
{
  return outer::common_type<
             outer::chrono::duration<int, int>,
             outer::chrono::duration<int, int> >::value == 1
             ? 0
             : 1;
}
