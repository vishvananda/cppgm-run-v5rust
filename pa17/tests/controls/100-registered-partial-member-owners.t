// expect exit 0
// Renamed template parameters still name a registered partial owner.
template<class T> struct Owner;

template<class T>
struct Owner<T *>
{
  Owner();
  ~Owner();
  operator int();
  template<class U> int value();
  static int count;
};

template<class U> int Owner<U *>::count = 0;
template<class U> Owner<U *>::Owner() { ++count; }
template<class U> Owner<U *>::~Owner() { --count; }
template<class U> Owner<U *>::operator int() { return count; }

template<class U>
template<class V>
int Owner<U *>::value()
{
  return sizeof(V);
}

int main()
{
  {
    Owner<char *> value;
    if (int(value) != 1 || value.value<int>() != 4)
      return 1;
  }
  return Owner<char *>::count;
}
