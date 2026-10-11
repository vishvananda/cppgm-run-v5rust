// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

template<class T>
struct static_box
{
  static int value();
};

template<class T>
int static_box<T>::value()
{
  return 9;
}

template class static_box<int>;

int main()
{
  return 0;
}
