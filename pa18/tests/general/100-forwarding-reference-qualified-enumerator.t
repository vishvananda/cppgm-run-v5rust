// Expected: a qualified enumerator is a prvalue during forwarding-reference
// deduction, so T&& should deduce T as the enum type, not enum&.

namespace N
{
enum E
{
  A
};
}

template<class T1, class T2>
int pick(T1&&, T2&&)
{
  return 0;
}

int main()
{
  int x = 0;
  return pick(x, N::A);
}
