template<class T>
struct arg {};

template<class T>
struct trait {
  static const bool value = true;
};

int main()
{
  static_assert(trait< ::arg<int> >::value, "rooted type template argument");
  return 0;
}
