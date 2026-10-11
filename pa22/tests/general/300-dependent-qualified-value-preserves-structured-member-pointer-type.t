template<int N>
struct int_ {
  static const int value = N;
};

template<int N>
struct arg {
};

template<class T>
struct add_pointer {
};

template<class T, class Transform>
struct parameter_types {
  typedef int_<32769> lower_bound_;
};

template<class Vector, int Offset>
struct vector_at {
  typedef int_<Vector::lower_bound_::value + Offset> type;
};

class C;

typedef C (C::*mem_func_ptr)();
typedef vector_at<parameter_types<mem_func_ptr, add_pointer<arg<-1> > >, 0>::type result;

int main()
{
  return result::value == 32769 ? 0 : 1;
}
