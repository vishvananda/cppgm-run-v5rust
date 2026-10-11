// VALIDATION: compile-pass
// N3485 focus: 14.5.3 [temp.variadic], 14.8.1 [temp.arg.explicit]
// Replaying an explicit type argument alongside a pack must keep the outer
// cv-reference declarator around its nested template-id.

template<class T>
struct token
{
  token(int);
};

struct partial
{
  int value;
};

template<class T, class... Signatures>
struct result;

template<class Completion, class... Signatures, class Initiation>
auto initiate(Initiation&& initiation, Completion& value)
  -> decltype(result<Completion, Signatures...>::call(
      static_cast<Initiation&&>(initiation),
      static_cast<Completion&&>(value)));

template<class Completion, class... Signatures>
struct result
{
  template<class Initiation, class RawToken>
  static int call(Initiation&&, RawToken&&);
};

template<class... Signatures>
struct result<partial, Signatures...>
{
  template<class Initiation, class RawToken>
  static auto call(Initiation&& initiation, RawToken&& raw)
    -> decltype(initiate<const token<Initiation>&, Signatures...>(
        static_cast<Initiation&&>(initiation),
        token<Initiation>(raw.value)));
};

int main()
{
  partial value;
  return initiate<partial, void(int)>(0, value);
}
