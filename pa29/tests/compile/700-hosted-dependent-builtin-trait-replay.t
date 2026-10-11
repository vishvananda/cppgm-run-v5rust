template<class T, T v>
struct integral_constant {
  typedef integral_constant type;
  typedef T value_type;
  static const T value = v;
};

template<class R, class F, class... Args>
static const bool constructible_v = __is_constructible(F, Args...);

template<class R, class F, class... Args>
struct constructible_probe
    : integral_constant<bool, constructible_v<R, F, Args...> > {};

template<class T>
struct has_virtual_destructor
    : integral_constant<bool, __has_virtual_destructor(T)> {};

template<class T>
struct is_abstract : integral_constant<bool, __is_abstract(T)> {};

template<class T>
struct is_polymorphic : integral_constant<bool, __is_polymorphic(T)> {};

template<class T>
struct is_literal_type : integral_constant<bool, __is_literal_type(T)> {};

template<class T>
struct rank : integral_constant<unsigned long, __array_rank(T)> {};

struct VirtualDtor {
  virtual ~VirtualDtor();
};

struct Abstract {
  virtual void f() = 0;
};

struct Concrete : Abstract {
  void f();
};

struct LiteralLike {
  int value;
};

typedef int Array2[2][3];
typedef constructible_probe<void, int> ForceVariableTemplateReplay;

static_assert(has_virtual_destructor<VirtualDtor>::type::value, "");
static_assert(is_abstract<Abstract>::type::value, "");
static_assert(!is_abstract<Concrete>::type::value, "");
static_assert(is_polymorphic<Abstract>::type::value, "");
static_assert(is_literal_type<LiteralLike>::type::value, "");
static_assert(rank<Array2>::type::value == 2, "");

int main()
{
  return 0;
}
