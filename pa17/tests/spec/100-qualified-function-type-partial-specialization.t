// N3485 focus: 14.5.5 [temp.class.spec]

namespace N
{
struct Binding {};
}

template<class Signature>
struct Function;

template<class R, class A>
struct Function<R(A)>
{
  R (*call)(A);
};

struct Hooks
{
  Function<N::Binding *(N::Binding *)> resolve;
};

int main()
{
  Hooks hooks;
  return hooks.resolve.call == 0 ? 0 : 1;
}
