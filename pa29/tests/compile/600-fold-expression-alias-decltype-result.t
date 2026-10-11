namespace library {

template<class... Types>
using sum_type = decltype((Types() + ...));

}

template<class A, class B>
library::sum_type<A, B> add(A a, B b)
{
  using result_type = library::sum_type<A, B>;
  return static_cast<result_type>(a + b);
}

int main()
{
  return add(1.0, 2.0f) == 3.0 ? 0 : 1;
}
