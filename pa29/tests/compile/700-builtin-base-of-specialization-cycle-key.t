template<class T, T Value>
struct integral_constant
{
    static const T value = Value;
    typedef integral_constant type;
};

template<class T, class U>
struct is_same : integral_constant<bool, false> {};

template<class T>
struct is_same<T, T> : integral_constant<bool, true> {};

template<class Base, class Derived>
struct is_base_and_derived
    : integral_constant<bool,
          __is_base_of(Base, Derived) && !is_same<Base, Derived>::value> {};

struct none {};

template<class First = none, class Second = none>
struct type_list {};

template<class First, class Second>
struct inherit2 : First, Second {};

template<bool Condition, class True, class False>
struct conditional
{
    typedef True type;
};

template<class True, class False>
struct conditional<false, True, False>
{
    typedef False type;
};

template<class Feature>
struct feature_of
{
    typedef Feature type;
};

template<class A, class B>
struct is_dependent_on
    : is_base_and_derived<typename feature_of<B>::type, A> {};

template<class Features>
struct depends_on_base;

template<>
struct depends_on_base<type_list<none, none> > {};

template<class First>
struct depends_on_base<type_list<First, none> > : First {};

template<class First, class Second>
struct depends_on_base<type_list<First, Second> >
    : conditional<is_dependent_on<First, Second>::value,
                  First,
                  inherit2<First, Second> >::type {};

template<class First = none, class Second = none>
struct depends_on : depends_on_base<type_list<First, Second> >
{
    typedef type_list<First, Second> dependencies;
};

struct window : depends_on<> {};
struct sum : depends_on<window> {};
struct count : depends_on<window> {};
struct mean : depends_on<sum, count> {};

static_assert(is_dependent_on<mean, window>::value, "transitive dependency");

int main()
{
    return 0;
}
