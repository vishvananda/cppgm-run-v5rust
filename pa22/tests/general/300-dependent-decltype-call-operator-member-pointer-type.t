// Regression: a dependent decltype of a call operator member-function pointer
// must resolve after the callable type is known so partial-specialization
// matching can recover the first argument type.

template<class T> struct remove_reference { typedef T type; };
template<class T> struct remove_reference<T &> { typedef T type; };
template<class T> struct remove_reference<T &&> { typedef T type; };

template<class T> struct first_arg;

template<class C, class R, class A1, class... A>
struct first_arg<R (C::*)(A1, A...) const>
{
  typedef A1 type;
};

template<class Json>
struct json_encoder
{
  explicit json_encoder(Json &) : touched(0) {}
  int touched;
};

struct json {};

struct first_callable
{
  void operator()(json_encoder<json> & enc) const
  {
    enc.touched = 7;
  }
};

template<class F1>
int invoke(F1 f1)
{
  typedef typename remove_reference<
      typename first_arg<decltype(&F1::operator())>::type
  >::type arg_type;
  json value;
  arg_type enc(value);
  f1(enc);
  return enc.touched;
}

int main()
{
  return invoke(first_callable()) == 7 ? 0 : 1;
}
