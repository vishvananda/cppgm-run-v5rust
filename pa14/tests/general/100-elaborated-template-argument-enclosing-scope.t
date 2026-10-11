template<class T>
struct wrapper
{
  typedef T type;
};

struct Outer : wrapper<struct ForwardInBase>
{};

struct ForwardInBase
{
  static const int value = 7;
};

int main()
{
  return Outer::type::value == 7 ? 0 : 1;
}
