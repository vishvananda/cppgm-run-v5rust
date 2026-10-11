// N3485 focus: [over.match.oper] enum operands enter operator lookup, but
// builtin operators remain valid when no user-defined candidate is viable.
namespace ns {
enum E { k = 3 };

struct tag {};

template<class T>
tag operator-(tag, T);
}

int main()
{
  unsigned long x = 5;
  return (x - ns::k) == 2 ? 0 : 1;
}
