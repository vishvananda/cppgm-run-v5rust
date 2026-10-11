namespace std { template<typename T> class initializer_list; }

template<class T>
struct identity {};

int total;

template<class T>
int touch(identity<T>)
{
  total = total + sizeof(T);
  return 0;
}

template<class... Ts>
int visit()
{
  auto values = { (touch(identity<Ts>()), 0)... };
  (void)values;
  return total;
}

int main()
{
  return visit<char, int>() == sizeof(char) + sizeof(int) ? 0 : 1;
}
