template<class Signature>
struct message;

template<class R>
struct message<R()> {
  static const int value = 0;
};

template<class R, class Arg0>
struct message<R(Arg0)> {
  static const int value = 1;
};

template<class R, class... Args>
struct message<R(Args...)> {
  static const int value = 2;
};

struct error_code {
};

static_assert(message<void(error_code)>::value == 1,
              "fixed-arity function partial is more specialized than pack");

int main()
{
  return 0;
}
