template<class T>
struct Cache
{
  static int table;
};

template<>
int Cache<int>::table = 9;
