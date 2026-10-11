template<class T>
struct base_ptr
{
  template<class U>
  base_ptr& operator=(base_ptr<U>&&)
  {
    return *this;
  }
};

template<class T>
struct shared_ptr : base_ptr<T>
{
  template<class U>
  shared_ptr& operator=(shared_ptr<U>&& other)
  {
    this->base_ptr<T>::operator=(static_cast<base_ptr<U>&&>(other));
    return *this;
  }
};

int main()
{
  shared_ptr<int> source;
  shared_ptr<const int> target;
  target = static_cast<shared_ptr<int>&&>(source);
  return 0;
}
