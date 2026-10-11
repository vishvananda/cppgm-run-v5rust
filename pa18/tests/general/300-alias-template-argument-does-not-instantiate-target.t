struct incomplete;

template<class, class U>
struct target { U member; };

template<class T>
using alias = target<T, incomplete>;

template<class>
struct consumer;

using identity = consumer<alias<int> >;

struct incomplete {};

int main()
{
  return 0;
}
