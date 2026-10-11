template<class T>
struct Box
{
  int target() const;
};

template<class T>
int Box<T>::target() const
{
  return 9;
}

template int Box<int>::target() const;
