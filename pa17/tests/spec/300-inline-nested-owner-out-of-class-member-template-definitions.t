// N3485 focus: 14.5.2 [temp.mem]

template<class T>
struct owner
{
  struct nested
  {
    int state;

    template<class U>
    nested(U value);

    template<class U>
    int add(U value);
  };
};

template<class T>
template<class U>
owner<T>::nested::nested(U value)
  : state(value)
{
}

template<class T>
template<class U>
int owner<T>::nested::add(U value)
{
  return state + value;
}

int main()
{
  owner<int>::nested value(2);
  return value.add(3) == 5 ? 0 : 1;
}
