// N3485 focus: 14.7.2 [temp.explicit]

template<class T>
struct box
{
  T value;

  void touch()
  {
  }

  template<class U>
  T add(U delta)
  {
    touch();
    return value + delta;
  }
};

extern template class box<int>;

int call_add(box<int> * p)
{
  return p->add<int>(2);
}
